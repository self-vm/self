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
    let from_bytes_handle = vm.memory.alloc(from_bytes_obj());

    fields.insert("from_byte".to_string(), Value::Handle(from_byte_handle));
    fields.insert("from_bytes".to_string(), Value::Handle(from_bytes_handle));
    MemObject::StructLiteral(StructLiteral::new("Buffer".to_string(), fields))
}

// from_byte
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

// from_bytes
fn from_bytes_obj() -> MemObject {
    MemObject::Function(Function::new(
        "from_bytes".to_string(),
        vec!["bytes vector".to_string()],
        Engine::Native(from_bytes),
    ))
}

pub fn from_bytes(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let bytes = params[0].as_vector_obj(vm)?;
    let bytes = bytes
        .elements
        .iter()
        .map(|v| -> Result<u8, VMError> {
            Ok(v.as_primitive(vm, DataType::Byte)?.as_byte(vm)?.value)
        })
        .collect::<Result<Vec<u8>, VMError>>()?;
    let buf = Buffer::new_initialized(bytes, vm);
    let buf_handle = vm
        .memory
        .alloc(MemObject::NativeStruct(NativeStruct::Buffer(buf)));

    Ok(Value::Handle(buf_handle))
}
