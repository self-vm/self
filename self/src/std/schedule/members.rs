use std::time::Duration;

use futures::future::BoxFuture;
use tokio::time::interval as tokio_interval;

use crate::{
    core::error::{self, type_errors::TypeError, VMError, VMErrorType},
    events::Event,
    memory::{Handle, MemObject},
    std::schedule::types::Interval,
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};

pub fn interval_obj() -> MemObject {
    MemObject::Function(Function::new(
        "interval".to_string(),
        vec!["callback".to_string(), "milliseconds".to_string()],
        Engine::NativeAsync(interval),
    ))
}

pub fn interval(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        if params.len() < 2 {
            return Err(error::throw(
                VMErrorType::TypeError(TypeError::InvalidArgsCount {
                    expected: 2,
                    received: params.len() as u32,
                }),
                vm,
            ));
        }

        let callback = &params[0].as_function_obj(vm)?;
        let callback = callback.clone();
        let milliseconds = &params[1].as_usize(vm)?;
        let vm_notifier = vm.get_vm_notifier();

        if debug {
            println!("INTERVAL -> {}", milliseconds)
        }

        let mut tick = tokio_interval(Duration::from_secs((milliseconds / 1000) as u64));
        let interval_struct = Interval::new_initialized(vm);
        let interval_obj_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Interval(
                interval_struct,
            )));

        tokio::spawn(async move {
            loop {
                tick.tick().await;
                vm_notifier.send(Event::Call(callback.clone()));
            }
        });

        Ok(Value::Handle(interval_obj_handle))
    })
}

// Interval type methods
pub fn start_obj() -> MemObject {
    MemObject::Function(Function::new(
        "start".to_string(),
        vec![],
        Engine::Native(start),
    ))
}

pub fn start(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    Ok(Value::RawValue(RawValue::Nothing))
}
