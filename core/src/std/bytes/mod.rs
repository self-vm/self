use std::collections::HashMap;

use crate::{
    core::error::{self, byte_errors::ByteError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    types::{
        object::{
            func::{Engine, Function},
            vector::Vector,
        },
        raw::{byte::Byte, RawValue},
        Value,
    },
    vm::Vm,
};

pub fn init_constructor() -> Function {
    Function::new(
        "Byte".to_string(),
        vec!["byte value".to_string()],
        Engine::Native(constructor),
    )
}

pub fn constructor(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let byte_value = params[0].as_isize(vm)?;
    if byte_value < 0 || byte_value > 255 {
        return Err(error::throw(
            VMErrorType::Byte(ByteError::OutOfBounds {
                received: byte_value,
            }),
            vm,
        ));
    }

    let byte = Value::RawValue(RawValue::Byte(Byte::new(byte_value as u8)));
    Ok(byte)
}
