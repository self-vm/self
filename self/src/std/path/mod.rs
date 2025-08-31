use std::path::PathBuf;

use crate::core::error::{self, VMError, VMErrorType};
use crate::memory::{MemObject, Handle};
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
        match value {
            Value::RawValue(v) => {
                return Err(error::throw(VMErrorType::TypeMismatch {
                    expected: "string".to_string(),
                    received: v.get_type_string(),
                }, vm))
            },
            Value::HeapRef(r) => {
                // let value = vm.resolve_heap_ref(r); 
                // match value {
                //     MemObject::String(s) => acc.push(s),
                //     _ => {
                //         return Err(error::throw(VMErrorType::TypeMismatch {
                //             expected: "string".to_string(),
                //             received: value.to_string(vm),
                //         }))
                //     }
                // }
                todo!()
            },
            Value::BoundAccess(_) => {
                return Err(error::throw(VMErrorType::TypeMismatch {
                    expected: "string".to_string(),
                    received: "bound_access".to_string(),
                }, vm));
            },
            Value::Handle(_) => {
                todo!()
            },
        }
    }
    Ok(Value::HeapRef(vm.heap.allocate(MemObject::String(
        acc.to_str().unwrap().to_string(),
    ))))
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
