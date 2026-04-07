use colored::Colorize;
use futures::future::BoxFuture;
use self_vm::core::error::VMError;
use self_vm::memory::{Handle, MemObject};
use self_vm::std::ai::members::{chain as ai_chain, unfold as ai_unfold};
use self_vm::std::generate_native_module;
use self_vm::std::NativeModule;
use self_vm::types::object::func::{Engine, Function};
use self_vm::types::object::native_struct::NativeStruct;
use self_vm::types::object::structs::StructLiteral;
use self_vm::types::Value;
use self_vm::vm::Vm;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex,
};

use tokio::io::{stdin, AsyncRead};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Global state shared with the unfold callback
// ---------------------------------------------------------------------------

static TUI_TX: Mutex<Option<mpsc::Sender<ChatMessage>>> = Mutex::new(None);
static TUI_STEP: AtomicU32 = AtomicU32::new(1);

// ---------------------------------------------------------------------------
// Unfold callback — unchanged from before
// ---------------------------------------------------------------------------

fn tui_unfold(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        let tx = TUI_TX.lock().unwrap().clone();
        let step = TUI_STEP.fetch_add(1, Ordering::Relaxed);

        let link = params[0].as_native_struct(vm)?.as_link(vm)?;

        let def = link
            .shape
            .property_access("def")
            .and_then(|v| v.as_string_obj(vm).ok())
            .unwrap_or_else(|| "...".to_string());

        if let Some(ref tx) = tx {
            let _ = tx.send(ChatMessage::Step(step, def)).await;
        }

        let resolved = if let Some(a_val) = link.shape.property_access("action") {
            if let Ok(action) = a_val.as_native_struct(vm).and_then(|ns| ns.as_action(vm)) {
                if let Some(ref tx) = tx {
                    let _ = tx
                        .send(ChatMessage::Action(
                            action.module.clone(),
                            action.member.clone(),
                        ))
                        .await;
                }
                let exec_fn_handle = action.exec.clone();
                if let MemObject::Function(exec_fn) = vm.memory.resolve(&exec_fn_handle) {
                    let exec_fn = exec_fn.clone();
                    let action_handle = vm
                        .memory
                        .alloc(MemObject::NativeStruct(NativeStruct::Action(action)));
                    let exec_result = vm
                        .run_function(&exec_fn, Some(action_handle), vec![], debug)
                        .await;
                    if let Some(e) = exec_result.error {
                        if let Some(ref tx) = tx {
                            let _ = tx
                                .send(ChatMessage::Error(format!("Action failed: {}", e.message)))
                                .await;
                        }
                        Value::RawValue(self_vm::types::raw::RawValue::Utf8(
                            self_vm::types::raw::utf8::Utf8::new(format!("Error: {}", e.message)),
                        ))
                    } else if let Some(r) = exec_result.result {
                        let res_str = 'preview: {
                            if let Value::Handle(ref h) = r {
                                if let MemObject::NativeStruct(NativeStruct::HttpResponse(
                                    ref resp,
                                )) = vm.memory.resolve(h)
                                {
                                    if let Some(Value::Handle(buf_h)) =
                                        resp.shape.property_access("body")
                                    {
                                        if let MemObject::NativeStruct(NativeStruct::Buffer(buf)) =
                                            vm.memory.resolve(&buf_h)
                                        {
                                            let text =
                                                String::from_utf8_lossy(&buf.bytes).to_string();
                                            const PREVIEW_CHARS: usize = 300;
                                            let preview: String =
                                                text.chars().take(PREVIEW_CHARS).collect();
                                            let suffix = if text.chars().count() > PREVIEW_CHARS {
                                                "…"
                                            } else {
                                                ""
                                            };
                                            break 'preview format!("{}{}", preview, suffix);
                                        }
                                    }
                                }
                            }
                            r.to_string(vm)
                        };
                        if res_str != "nothing" {
                            if let Some(ref tx) = tx {
                                let _ = tx.send(ChatMessage::ActionResult(res_str)).await;
                            }
                        }
                        r
                    } else {
                        Value::RawValue(self_vm::types::raw::RawValue::Nothing)
                    }
                } else {
                    Value::RawValue(self_vm::types::raw::RawValue::Nothing)
                }
            } else {
                Value::RawValue(self_vm::types::raw::RawValue::Nothing)
            }
        } else {
            Value::RawValue(self_vm::types::raw::RawValue::Nothing)
        };

        let mut fields = HashMap::new();
        fields.insert(
            "continue".to_string(),
            Value::RawValue(self_vm::types::raw::RawValue::Bool(
                self_vm::types::raw::bool::Bool::new(true),
            )),
        );
        fields.insert("resolved".to_string(), resolved);
        let result_struct = StructLiteral::new("UnfoldResult".to_string(), fields, vm);
        let result_handle = vm.memory.alloc(MemObject::StructLiteral(result_struct));
        Ok(Value::Handle(result_handle))
    })
}

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum ChatMessage {
    Thinking,
    Step(u32, String),
    Action(String, String),
    ActionResult(String),
    GoalReached(String),
    Error(String),
    Quit,
}

// ---------------------------------------------------------------------------
// Chat command
// ---------------------------------------------------------------------------

pub struct Chat {
    args: Vec<String>,
}

impl Chat {
    pub fn new(args: Vec<String>) -> Chat {
        Chat { args }
    }

    pub async fn exec(&self) {
        self.load_env();

        // VM setup
        let mut vm = self_vm::new(vec![]);
        vm.run(&vec![]).await;

        let (ai_module_name, ai_fields) = generate_native_module(NativeModule::AI);
        let mut fields = HashMap::new();
        for (name, obj) in ai_fields {
            let handle = vm.memory.alloc(obj);
            fields.insert(name, Value::Handle(handle));
        }
        let ai_struct = self_vm::types::object::structs::StructLiteral::new(
            ai_module_name.clone(),
            fields,
            &mut vm,
        );
        let ai_struct_handle = vm.memory.alloc(MemObject::StructLiteral(ai_struct));
        vm.call_stack
            .put_to_frame(ai_module_name, Value::Handle(ai_struct_handle));

        // Logo
        Self::print_logo();

        // Agent channel — the VM lives inside the spawned task
        let (input_tx, mut input_rx) = mpsc::channel::<String>(10);
        let (msg_tx, mut msg_rx) = mpsc::channel::<ChatMessage>(100);

        let agent_handle = tokio::spawn(async move {
            let mut vm = vm;
            while let Some(prompt) = input_rx.recv().await {
                Self::run_agent(&mut vm, &prompt, msg_tx.clone()).await;
            }
        });

        // If the binary was called with a prompt as argument, run it first
        if !self.args.is_empty() {
            let prompt = self.args.join(" ");
            if !prompt.trim().is_empty() {
                println!("\n  {} {}\n", "❯".magenta().bold(), prompt.white());
                let _ = input_tx.send(prompt).await;
                while let Some(msg) = msg_rx.recv().await {
                    let done = matches!(msg, ChatMessage::Quit);
                    Self::print_message(&msg);
                    if done {
                        break;
                    }
                }
            }
        }

        // Interactive input loop
        let mut reader = BufReader::new(stdin());
        loop {
            print!("\n  {} ", "❯".magenta().bold());
            std::io::stdout().flush().unwrap();

            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break, // EOF / pipe closed
                Ok(_) => {}
            }

            let input = line.trim().to_string();
            if input == "exit" || input == "quit" {
                break;
            }
            if input.is_empty() {
                continue;
            }

            println!();
            let _ = input_tx.send(input).await;

            while let Some(msg) = msg_rx.recv().await {
                let done = matches!(msg, ChatMessage::Quit);
                Self::print_message(&msg);
                if done {
                    break;
                }
            }
        }

        drop(input_tx);
        let _ = agent_handle.await;
    }

    fn print_logo() {
        // Gradient from blue to lighter blue
        let lines = [
            ("    ██████▓▓▒▒▒▒▒░░░░    ", 102u8),
            ("     ██████▓▒▒░░         ", 119),
            ("      ██████▒▒░░          ", 136),
            ("     ░▒▓█████▓▒▒░         ", 153),
            ("       ░░▒▒▓█████▓░       ", 170),
            ("         ░░▒▒▓█████▓░     ", 153),
            ("     ░░░░░▒▒▒▒▒▓██████    ", 136),
        ];
        println!();
        for (line, g) in &lines {
            // RGB: R=0, G=gradient, B=255
            println!("  \x1b[38;2;0;{};255m{}\x1b[0m", g, line);
        }
        println!();
        println!("  {}", "S E L F   V M".dimmed());
        println!("  {}", "─".repeat(40).dimmed());
        println!("  {}", "https://github.com/self-vm/self".dimmed());
        println!("  {}", "─".repeat(40).dimmed());
        println!();
    }

    fn print_message(msg: &ChatMessage) {
        match msg {
            ChatMessage::Thinking => {
                println!("  {} {}", "󰚩", "Thinking...".dimmed().italic());
            }
            ChatMessage::Step(i, d) => {
                println!("  {} Step {}: {}", "󰋗".cyan(), i, d.cyan().bold());
            }
            ChatMessage::Action(m, mem) => {
                println!("    {} {}", "➜".dimmed(), format!("{}.{}", m, mem).dimmed());
            }
            ChatMessage::ActionResult(r) => {
                for line in r.lines() {
                    println!("      {} {}", "│".dimmed(), line.dimmed());
                }
            }
            ChatMessage::GoalReached(r) => {
                println!();
                println!("  {} {}", "✨", "Goal Reached".green().bold());
                for line in r.lines() {
                    println!("    {}", line.white());
                }
                println!();
            }
            ChatMessage::Error(e) => {
                println!("  {} {}", "󰅚".red(), format!("Error: {}", e).red());
            }
            ChatMessage::Quit => {}
        }
    }

    // -----------------------------------------------------------------------
    // Agent loop — unchanged
    // -----------------------------------------------------------------------

    async fn run_agent(vm: &mut Vm, prompt: &str, tx: mpsc::Sender<ChatMessage>) {
        let _ = tx.send(ChatMessage::Thinking).await;

        *TUI_TX.lock().unwrap() = Some(tx.clone());
        TUI_STEP.store(1, Ordering::Relaxed);

        let purpose_str = self_vm::types::object::string::SelfString::new(prompt.to_string(), vm);
        let purpose_handle = vm.memory.alloc(MemObject::String(purpose_str));
        let end_cond_str = self_vm::types::object::string::SelfString::new(
            "user objective reached".to_string(),
            vm,
        );
        let end_cond_handle = vm.memory.alloc(MemObject::String(end_cond_str));

        let chain_val = ai_chain(
            vm,
            None,
            vec![
                Value::Handle(purpose_handle),
                Value::Handle(end_cond_handle),
            ],
            false,
        )
        .await;

        let chain_handle = match chain_val {
            Ok(v) => match v.as_handle() {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(ChatMessage::Error(e.message)).await;
                    let _ = tx.send(ChatMessage::Quit).await;
                    *TUI_TX.lock().unwrap() = None;
                    return;
                }
            },
            Err(e) => {
                let _ = tx.send(ChatMessage::Error(e.message)).await;
                let _ = tx.send(ChatMessage::Quit).await;
                *TUI_TX.lock().unwrap() = None;
                return;
            }
        };

        let callback_fn = Function::new(
            "tui_unfold".to_string(),
            vec!["link".to_string()],
            Engine::NativeAsync(tui_unfold),
        );
        let callback_handle = vm.memory.alloc(MemObject::Function(callback_fn));

        let result = ai_unfold(
            vm,
            Some(chain_handle),
            vec![Value::Handle(callback_handle)],
            false,
        )
        .await;

        *TUI_TX.lock().unwrap() = None;

        match result {
            Ok(r) => {
                let _ = tx.send(ChatMessage::GoalReached(r.to_string(vm))).await;
                let _ = tx.send(ChatMessage::Quit).await;
            }
            Err(e) => {
                let _ = tx.send(ChatMessage::Error(e.message)).await;
                let _ = tx.send(ChatMessage::Quit).await;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Env — unchanged
    // -----------------------------------------------------------------------

    fn load_env(&self) {
        if dotenvy::dotenv().is_err() {
            if let Ok(home) = std::env::var("HOME") {
                let path = std::path::PathBuf::from(home).join(".self").join(".env");
                if path.exists() {
                    let _ = dotenvy::from_path(path);
                }
            }
        }
    }
}
