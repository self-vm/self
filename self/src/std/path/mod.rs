use std::path::PathBuf;

use crate::core::error::{self, VMError, VMErrorType};
use crate::memory::{Handle, MemObject};
use crate::std::heap_utils::put_string;
use crate::std::{NativeMember, NativeModuleDef};
use crate::types::object::func::{Engine, Function};
use crate::types::Value;
use crate::vm::Vm;

fn join(
    vm: &mut Vm,
    _self: Option<Handle>,
    _params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let mut acc = PathBuf::new();
    for value in _params {
        let value = value.as_string_obj(vm)?;
        acc.push(value);
    }
    Ok(Value::Handle(put_string(
        vm,
        acc.to_str().unwrap().to_string(),
    )))
}

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    (
        "path".to_string(),
        vec![(
            "join".to_string(),
            MemObject::Function(Function::new(
                "join".to_string(),
                vec!["...path_segment".to_string()],
                Engine::Native(join),
            )),
        )],
    )
}

pub fn generate_mod_def() -> NativeModuleDef {
    NativeModuleDef {
        module: "path".to_string(),
        members: vec![NativeMember {
            name: "join".to_string(),
            description: "merges an arbitrary number of paths into a single path".to_string(),
            params: Some(vec!["...path_segment".to_string()]),
        }],
    }
}
