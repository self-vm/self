use std::collections::HashMap;

use crate::{
    core::error::{self, byte_errors::ByteError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    opcodes::DataType,
    std::buffer::types::Buffer,
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            structs::StructLiteral,
        },
        raw::{byte::Byte, RawValue},
        Value,
    },
    vm::Vm,
};

pub mod types;

pub fn init_constructor(vm: &mut Vm) -> MemObject {
    let mut fields = HashMap::new();
    let from_byte_handle = vm.memory.alloc(from_byte_obj());

    fields.insert("from_byte".to_string(), Value::Handle(from_byte_handle));
    MemObject::StructLiteral(StructLiteral::new("Buffer".to_string(), fields))
}

fn from_byte_obj() -> MemObject {
    MemObject::Function(Function::new(
        "from_byte".to_string(),
        vec!["byte".to_string()],
        Engine::Native(from_byte),
    ))
}

pub fn from_byte(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let byte = params[0].as_primitive(vm, DataType::Byte)?.as_byte(vm)?;
    let buf = Buffer::new_initialized(vec![byte.value], vm);
    let buf_handle = vm
        .memory
        .alloc(MemObject::NativeStruct(NativeStruct::Buffer(buf)));

    Ok(Value::Handle(buf_handle))
}
