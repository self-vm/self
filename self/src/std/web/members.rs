use std::{thread::sleep, time::Duration};

use chromiumoxide::{Browser as ChromiumBrowser, BrowserConfig};
use futures::future::BoxFuture;
use futures::StreamExt;

use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{web::types::Browser, NativeMember},
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            string::SelfString,
        },
        raw::{u32::U32, RawValue},
        Value,
    },
    vm::Vm,
};

pub fn browser_def() -> NativeMember {
    NativeMember {
        name: "browser".to_string(),
        description: "creates a new browser to be able to navigate through the internet"
            .to_string(),
        params: None,
    }
}

pub fn browser_obj() -> MemObject {
    MemObject::Function(Function::new(
        "browser".to_string(),
        vec![],
        Engine::NativeAsync(browser),
    ))
}

fn browser(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    Box::pin(async move {
        let browser_obj = Browser::new_initialized(vm);
        let browser_obj_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Browser(browser_obj)));
        Ok(Value::Handle(browser_obj_handle))
    })
}
