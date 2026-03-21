use std::collections::HashMap;

use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::buffer::types::{as_string_obj, Buffer},
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            structs::StructLiteral,
        },
        Value,
    },
    vm::Vm,
};
use self_bytecode::DataType;

pub mod types;

pub fn init_constructor(vm: &mut Vm) -> MemObject {
    let mut fields = HashMap::new();
    let from_byte_handle = vm.memory.alloc(from_byte_obj());
    let from_bytes_handle = vm.memory.alloc(from_bytes_obj());

    fields.insert("from_byte".to_string(), Value::Handle(from_byte_handle));
    fields.insert("from_bytes".to_string(), Value::Handle(from_bytes_handle));
    MemObject::StructLiteral(StructLiteral::new("Buffer".to_string(), fields, vm))
}

pub fn init_lib() -> Vec<(String, MemObject)> {
    let mut fields = vec![];

    fields.push(("as_string".to_string(), as_string_obj()));

    fields
}

pub fn add_handlers(vm: &mut Vm) -> HashMap<String, Value> {
    let mut loaded_members = HashMap::new();

    // if strings lib members are already loaded
    if vm.handlers.contains_key("buffer.as_string") {
        if let Some(mem) = vm.get_handler("buffer.as_string") {
            loaded_members.insert("as_string".to_string(), Value::Handle(mem));
        }
    } else {
        let fields = init_lib();
        for (handler_name, handler_obj) in fields {
            let obj_handle = vm.memory.alloc(handler_obj);
            loaded_members.insert(handler_name.clone(), Value::Handle(obj_handle.clone()));

            let handler_name = format!("string.{}", handler_name); // add lib prefix
            vm.handlers.insert(handler_name, obj_handle);
        }
    }

    return loaded_members;
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
