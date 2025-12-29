use std::collections::HashMap;

use libloading::Library;

use crate::{
    memory::MemObject,
    std::native::members::call,
    types::{
        object::{
            func::{Engine, Function},
            structs::StructLiteral,
        },
        raw::{utf8::Utf8, RawValue},
        Value,
    },
    vm::Vm,
};

#[derive(Debug)]
pub struct NativeLib {
    pub library: Library,
    pub path: String,
    pub shape: StructLiteral,
}

impl NativeLib {
    pub fn new_initialized(path: String, library: Library, vm: &mut Vm) -> NativeLib {
        // todo: we should here use a memory load type for type specific functions
        // the same applies to all native types like Action. This custom load
        // function will load the function if it doesnt already exists in memory
        let call_function = vm.memory.alloc(MemObject::Function(Function::new(
            "call".to_string(),
            vec![],
            Engine::Native(call),
        )));

        let mut fields = HashMap::new();
        fields.insert("call".to_string(), Value::Handle(call_function));

        NativeLib {
            path,
            library,
            shape: StructLiteral::new("NativeLib".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        format!("NativeLib({})", self.path)
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        match property {
            "path" => Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                self.path.clone(),
            )))),
            _ => self.shape.property_access(property),
        }
    }
}
