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
use std::io::{self, stdout};
use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;
use tokio::sync::mpsc;

static TUI_TX: Mutex<Option<mpsc::Sender<ChatMessage>>> = Mutex::new(None);
static TUI_STEP: AtomicU32 = AtomicU32::new(1);

fn tui_unfold(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        // Clone sender before any await point so we don't hold the lock across awaits
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
                        let res_str = r.to_string(vm);
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

        // Return { continue: true, resolved: result } matching what unfold() expects
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

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

#[derive(Clone)]
enum ChatMessage {
    Logo(Vec<String>),
    User(String),
    System(String),
    Thinking,
    Step(u32, String),
    Action(String, String),
    ActionResult(String),
    GoalReached(String),
    Error(String),
}

struct App {
    input: String,
    messages: Vec<ChatMessage>,
    list_state: ListState,
    should_quit: bool,
}

impl App {
    fn new() -> App {
        App {
            input: String::new(),
            messages: vec![],
            list_state: ListState::default(),
            should_quit: false,
        }
    }

    fn push_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        let len = self.messages.len();
        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }
}

pub struct Chat {
    args: Vec<String>,
}

impl Chat {
    pub fn new(args: Vec<String>) -> Chat {
        Chat { args }
    }

    pub async fn exec(&self) {
        self.load_env();

        // Setup terminal
        enable_raw_mode().unwrap();
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        // Create app state
        let mut app = App::new();

        // Initial Logo and Welcome in messages
        let logo = vec![
            "    ██████▓▓▒▒▒▒▒░░░░    ".to_string(),
            "     ██████▓▒▒░░         ".to_string(),
            "      ██████▒▒░░          ".to_string(),
            "     ░▒▓█████▓▒▒░         ".to_string(),
            "       ░░▒▒▓█████▓░       ".to_string(),
            "         ░░▒▒▓█████▓░     ".to_string(),
            "     ░░░░░▒▒▒▒▒▓██████    ".to_string(),
        ];
        app.push_message(ChatMessage::Logo(logo));
        app.push_message(ChatMessage::System(" ".to_string()));
        app.push_message(ChatMessage::System("S E L F   V M".to_string()));
                app.push_message(ChatMessage::System("─".repeat(40)));
        app.push_message(ChatMessage::System(
            "https://github.com/self-vm/self".to_string(),
        ));
        app.push_message(ChatMessage::System("─".repeat(40)));
        app.push_message(ChatMessage::System(" ".to_string()));

        // Setup VM
        let mut vm = self_vm::new(vec![]);
        vm.run(&vec![]).await;

        let (ai_module_name, ai_fields) = generate_native_module(NativeModule::AI);
        let mut fields = std::collections::HashMap::new();
        for (name, obj) in ai_fields {
            let handle = vm.memory.alloc(obj);
            fields.insert(name, Value::Handle(handle));
        }
        let ai_struct = self_vm::types::object::structs::StructLiteral::new(ai_module_name.clone(), fields, &mut vm);
        let ai_struct_handle = vm.memory.alloc(MemObject::StructLiteral(ai_struct));
        vm.call_stack
            .put_to_frame(ai_module_name, Value::Handle(ai_struct_handle));

        // Background agent channel
        let (tx, mut rx) = mpsc::channel(100);
        let (input_tx, mut input_rx) = mpsc::channel::<String>(10);

        // Handle initial prompt if present
        if !self.args.is_empty() {
            let initial_prompt = self.args.join(" ");
            if !initial_prompt.trim().is_empty() {
                app.push_message(ChatMessage::User(initial_prompt.clone()));
                let _ = input_tx.try_send(initial_prompt);
            }
        }

        // Main Loop
        let mut last_tick = std::time::Instant::now();
        let tick_rate = Duration::from_millis(100);

        // We'll manage the VM and agent loop here
        // Since the UI needs to be responsive, we run the agent in a separate task
        // But the agent needs access to the VM.
        // We'll move the VM into a background task and communicate via channels.

        // Move VM and agent logic to background task
        let agent_handle = tokio::spawn(async move {
            let mut vm = vm;
            while let Some(prompt) = input_rx.recv().await {
                Self::run_agent(&mut vm, &prompt, tx.clone()).await;
            }
        });

        loop {
            terminal.draw(|f| self.ui(f, &mut app)).unwrap();

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    match key.code {
                        KeyCode::Char(c) => {
                            if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                                app.should_quit = true;
                            } else {
                                app.input.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Enter => {
                            let input = app.input.drain(..).collect::<String>();
                            if input == "exit" || input == "quit" {
                                app.should_quit = true;
                            } else if !input.trim().is_empty() {
                                app.push_message(ChatMessage::User(input.clone()));
                                let _ = input_tx.send(input).await;
                            }
                        }
                        KeyCode::Esc => {
                            app.should_quit = true;
                        }
                        _ => {}
                    }
                }
            }

            // Receive messages from agent
            while let Ok(msg) = rx.try_recv() {
                app.push_message(msg);
            }

            if app.should_quit {
                break;
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = std::time::Instant::now();
            }
        }

        // Cleanup terminal
        disable_raw_mode().unwrap();
        execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
        terminal.show_cursor().unwrap();
    }

    fn ui(&self, f: &mut Frame, app: &mut App) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(f.size());

        // Messages List
        let messages: Vec<ListItem> = app
            .messages
            .iter()
            .map(|m| match m {
                ChatMessage::Logo(lines) => {
                    let width = chunks[0].width as usize;
                    let mut spans_lines = lines
                        .iter()
                        .enumerate()
                        .map(|(i, line)| {
                            let color = match i {
                                0 => Color::Rgb(0, 102, 255),
                                1 => Color::Rgb(0, 119, 255),
                                2 => Color::Rgb(0, 136, 255),
                                3 => Color::Rgb(0, 153, 255),
                                4 => Color::Rgb(0, 170, 255),
                                5 => Color::Rgb(0, 153, 255),
                                6 => Color::Rgb(0, 136, 255),
                                _ => Color::Rgb(0, 119, 255),
                            };
                            let line_width = line.chars().count();
                            let padding = (width.saturating_sub(line_width)) / 2;
                            Line::from(vec![
                                Span::raw(" ".repeat(padding)),
                                Span::styled(line, Style::default().fg(color)),
                            ])
                        })
                        .collect::<Vec<_>>();
                    ListItem::new(Text::from(spans_lines))
                }
                ChatMessage::User(u) => {
                    let text = Text::from(vec![Line::from(vec![
                        Span::styled(
                            "❯ ",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(u, Style::default().fg(Color::White)),
                    ])]);
                    ListItem::new(text)
                }
                ChatMessage::System(s) => {
                    let width = chunks[0].width as usize;
                    let s_width = s.chars().count();
                    let padding = (width.saturating_sub(s_width)) / 2;
                    ListItem::new(Line::from(vec![
                        Span::raw(" ".repeat(padding)),
                        Span::styled(s, Style::default().fg(Color::DarkGray)),
                    ]))
                }
                ChatMessage::Thinking => {
                    ListItem::new(Text::from(vec![Line::from(vec![Span::styled(
                        "󰚩 Thinking...",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )])]))
                }
                ChatMessage::Step(i, d) => {
                    ListItem::new(Text::from(vec![Line::from(vec![Span::styled(
                        format!("󰋗 Step {}: {}", i, d),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )])]))
                }
                ChatMessage::Action(m, mem) => {
                    ListItem::new(Text::from(vec![Line::from(vec![Span::styled(
                        format!("  ➜ {}.{}", m, mem),
                        Style::default().fg(Color::DarkGray),
                    )])]))
                }
                ChatMessage::ActionResult(r) => {
                    let mut lines = vec![];
                    for line in r.lines() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("    │ {}", line),
                            Style::default().fg(Color::DarkGray),
                        )]));
                    }
                    ListItem::new(Text::from(lines))
                }
                ChatMessage::GoalReached(r) => {
                    let mut lines = vec![Line::from(vec![Span::styled(
                        "✨ Goal Reached",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )])];
                    for line in r.lines() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::White),
                        )]));
                    }
                    ListItem::new(Text::from(lines))
                }
                ChatMessage::Error(e) => {
                    ListItem::new(Text::from(vec![Line::from(vec![Span::styled(
                        format!("󰅚 Error: {}", e),
                        Style::default().fg(Color::Red),
                    )])]))
                }
            })
            .collect();

        let messages_list = List::new(messages)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(Style::default())
            .highlight_symbol("");

        f.render_stateful_widget(messages_list, chunks[0], &mut app.list_state);

        // Input Box
        let input = Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Hi, friend.")
                    .border_style(Style::default().fg(Color::White)),
            );
        f.render_widget(input, chunks[1]);

        // Put cursor at the end of input
        f.set_cursor(chunks[1].x + app.input.len() as u16 + 1, chunks[1].y + 1);
    }

    async fn run_agent(vm: &mut Vm, prompt: &str, tx: mpsc::Sender<ChatMessage>) {
        let _ = tx.send(ChatMessage::Thinking).await;

        *TUI_TX.lock().unwrap() = Some(tx.clone());
        TUI_STEP.store(1, Ordering::Relaxed);

        // Allocate purpose and end_condition in VM memory
        let purpose_str =
            self_vm::types::object::string::SelfString::new(prompt.to_string(), vm);
        let purpose_handle = vm.memory.alloc(MemObject::String(purpose_str));
        let end_cond_str = self_vm::types::object::string::SelfString::new(
            "user objective reached".to_string(),
            vm,
        );
        let end_cond_handle = vm.memory.alloc(MemObject::String(end_cond_str));

        // Build chain (generates the master link)
        let chain_val = ai_chain(
            vm,
            None,
            vec![Value::Handle(purpose_handle), Value::Handle(end_cond_handle)],
            false,
        )
        .await;

        let chain_handle = match chain_val {
            Ok(v) => match v.as_handle() {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(ChatMessage::Error(e.message)).await;
                    *TUI_TX.lock().unwrap() = None;
                    return;
                }
            },
            Err(e) => {
                let _ = tx.send(ChatMessage::Error(e.message)).await;
                *TUI_TX.lock().unwrap() = None;
                return;
            }
        };

        // Register callback and run the unfold loop
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
            }
            Err(e) => {
                let _ = tx.send(ChatMessage::Error(e.message)).await;
            }
        }
    }

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
