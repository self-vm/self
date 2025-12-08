use std::{thread::sleep, time::Duration};

use chromiumoxide::{Browser, BrowserConfig};
use futures::future::BoxFuture;
use futures::StreamExt;

use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::NativeMember,
    types::{
        object::{
            func::{Engine, Function},
            string::SelfString,
        },
        raw::{u32::U32, RawValue},
        Value,
    },
    vm::Vm,
};

pub fn open_def() -> NativeMember {
    NativeMember {
        name: "open".to_string(),
        description: "open the given url in the params and return the html of the site".to_string(),
        params: Some(vec!["url(string)".to_string()]),
    }
}

pub fn open_obj() -> MemObject {
    MemObject::Function(Function::new(
        "open".to_string(),
        vec![],
        Engine::NativeAsync(open),
    ))
}

fn open(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
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

        // spawn a browser with window
        let (browser, mut handler) =
            Browser::launch(BrowserConfig::builder().with_head().build().map_err(|e| {
                error::throw(VMErrorType::Any("cannot open browser".to_string()), vm)
            })?)
            .await
            .map_err(|e| error::throw(VMErrorType::Any("cannot open browser".to_string()), vm))?;

        // CDP events here
        let pump = tokio::spawn(async move {
            while let Some(_ev) = handler.next().await {
                // log events here
            }
        });

        // new page and navigate
        let page = browser
            .new_page(url)
            .await
            .map_err(|e| error::throw(VMErrorType::Any("cannot open browser".to_string()), vm))?;
        page.wait_for_navigation()
            .await
            .map_err(|e| error::throw(VMErrorType::Any("cannot open browser".to_string()), vm))?;

        let title = page
            .get_title()
            .await
            .map_err(|e| error::throw(VMErrorType::Any("cannot open browser".to_string()), vm))?;

        let content = page
            .content()
            .await
            .map_err(|e| error::throw(VMErrorType::Any("cannot open browser".to_string()), vm))?;

        if debug {
            println!("WEB.open -> {:#?}", title);
        }

        pump.abort();

        let content_obj = SelfString::new(content, vm);
        let html_handle = vm.memory.alloc(MemObject::String(content_obj));
        Ok(Value::Handle(html_handle))
    })
}
