use std::{collections::HashMap, thread::sleep, time::Duration};

use chromiumoxide::{Browser as ChromiumBrowser, BrowserConfig, Handler};
use futures::future::BoxFuture;

use crate::{
    core::{
        error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
        handlers,
    },
    memory::{Handle, MemObject},
    std::{ai::types::SessionEnd, NativeMember, NativeModuleDef, NativeStructDef},
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            string::SelfString,
            structs::StructLiteral,
        },
        raw::{bool::Bool, u32::U32, RawValue},
        Value,
    },
    vm::Vm,
};

use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub struct Browser {
    tx: mpsc::Sender<BrowserCmd>,
    pub sessionable: bool,
    pub shape: StructLiteral,
}

#[derive(Debug)]
enum BrowserCmd {
    Open {
        url: String,
        resp: oneshot::Sender<Result<String, String>>,
    },
}

impl Browser {
    pub fn new_initialized(vm: &mut Vm) -> Browser {
        let mut fields = HashMap::new();

        let open_def_handle = vm.memory.alloc(open_obj());
        let close_def_handle = vm.memory.alloc(close_obj());

        fields.insert("open".to_string(), Value::Handle(open_def_handle));
        fields.insert("close".to_string(), Value::Handle(close_def_handle));

        let (tx, mut rx) = tokio::sync::mpsc::channel::<BrowserCmd>(32);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("browser rt");

            let local = tokio::task::LocalSet::new();

            local.block_on(&rt, async move {
                let (browser, mut handler) = ChromiumBrowser::launch(
                    BrowserConfig::builder()
                        .with_head()
                        .build()
                        .expect("browser config"),
                )
                .await
                .expect("launch chromium");

                // pump handler en local
                tokio::task::spawn_local(async move {
                    use futures::StreamExt;
                    while let Some(_ev) = handler.next().await {
                        // opcional
                    }
                });

                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        BrowserCmd::Open { url, resp } => {
                            let out: Result<String, String> = async {
                                // Más robusto que new_page(url)+wait_for_navigation
                                let page = browser
                                    .new_page("about:blank")
                                    .await
                                    .map_err(|e| format!("new_page: {e:?}"))?;

                                page.goto(url).await.map_err(|e| format!("goto: {e:?}"))?;

                                let html = page
                                    .content()
                                    .await
                                    .map_err(|e| format!("content: {e:?}"))?;

                                Ok(html)
                            }
                            .await;

                            let _ = resp.send(out);
                        }
                    }
                }
            });
        });

        Browser {
            tx,
            sessionable: true,
            shape: StructLiteral::new("Browser".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Browser {}".to_string()
    }

    // the runtime name serves as the way of exposing
    // the name of the actual struct in the callstack.
    pub fn get_defs(&self, runtime_name: &str) -> NativeStructDef {
        NativeStructDef {
            struct_name: runtime_name.to_string(),
            members: vec![open_def(), close_def()],
        }
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        match property {
            "is_sessionable" => Some(Value::RawValue(RawValue::Bool(Bool::new(self.sessionable)))),
            _ => self.shape.property_access(property),
        }
    }
}

// browser methods
pub fn open_def() -> NativeMember {
    NativeMember {
        name: "open".to_string(),
        description: "open the given url on the active browser and get its content".to_string(),
        params: Some(vec!["url(string)".to_string()]),
    }
}

pub fn open_obj() -> MemObject {
    MemObject::Function(Function::new(
        "open".to_string(),
        vec!["url(string)".to_string()],
        Engine::NativeAsync(open),
    ))
}

fn open(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        // resolve 'self'
        let (_self, _self_ref) = if let Some(_this) = _self {
            if let MemObject::NativeStruct(NativeStruct::Browser(b)) = vm.memory.resolve(&_this) {
                (b, _this)
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        };

        // resolve params
        if params.len() < 1 {
            return Err(error::throw(
                VMErrorType::TypeError(TypeError::InvalidArgsCount {
                    expected: 1,
                    received: params.len() as u32,
                }),
                vm,
            ));
        }

        let url = &params[0].as_string_obj(vm)?;

        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        _self
            .tx
            .send(BrowserCmd::Open {
                url: url.to_string(),
                resp: resp_tx,
            })
            .await
            .map_err(|_| error::throw(VMErrorType::Any("browser thread down".to_string()), vm))?;

        let html = resp_rx
            .await
            .map_err(|_| {
                error::throw(VMErrorType::Any("browser response dropped".to_string()), vm)
            })?
            .map_err(|e| error::throw(VMErrorType::Any(e), vm))?;

        if debug {
            println!("BROWSER.open -> {} bytes", html.len());
        }

        let content_obj = SelfString::new(html, vm);
        let html_handle = vm.memory.alloc(MemObject::String(content_obj));
        Ok(Value::Handle(html_handle))
    })
}

pub fn close_def() -> NativeMember {
    NativeMember {
        name: "close".to_string(),
        description: "close the browser session.".to_string(),
        params: None,
    }
}

pub fn close_obj() -> MemObject {
    MemObject::Function(Function::new(
        "open".to_string(),
        vec![],
        Engine::NativeAsync(close),
    ))
}

fn close(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        let session_end_obj = SessionEnd::new();
        let session_end_obj_handle =
            vm.memory
                .alloc(MemObject::NativeStruct(NativeStruct::SessionEnd(
                    session_end_obj,
                )));
        Ok(Value::Handle(session_end_obj_handle))
    })
}
