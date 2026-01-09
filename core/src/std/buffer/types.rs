use std::collections::HashMap;

use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            string::SelfString,
            structs::StructLiteral,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};

// this struct can be used as constructor
// or instance. both are valid
#[derive(Debug)]
pub struct Buffer {
    pub bytes: Vec<u8>,
    pub shape: StructLiteral,
}

impl Buffer {
    pub fn new_initialized(bytes: Vec<u8>, vm: &mut Vm) -> Buffer {
        let mut fields = HashMap::new();
        let as_string_handle = vm.memory.alloc(MemObject::Function(Function::new(
            "as_string".to_string(),
            vec![],
            Engine::Native(as_string),
        )));

        fields.insert("as_string".to_string(), Value::Handle(as_string_handle));
        let shape = StructLiteral::new("Buffer".to_string(), fields);

        Buffer { bytes, shape }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Buffer {}".to_string()
    }
}

fn as_string(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let (_self, _self_ref) = if let Some(_this) = _self {
        if let MemObject::NativeStruct(NativeStruct::Buffer(buf)) = vm.memory.resolve(&_this) {
            (buf, _this)
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let string_value = String::from_utf8(_self.bytes.clone());
    if let Ok(v) = string_value {
        let string_obj = SelfString::new(v, vm);
        let string_handle = vm.memory.alloc(MemObject::String(string_obj));
        Ok(Value::Handle(string_handle))
    } else {
        Ok(Value::RawValue(RawValue::Nothing))
    }
}
