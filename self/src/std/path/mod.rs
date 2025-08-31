use std::path::PathBuf;

use crate::core::error::{self, VMError, VMErrorType};
use crate::heap::{HeapObject, HeapRef};
use crate::std::{NativeMember, NativeModuleDef};
use crate::types::object::func::{Engine, Function};
use crate::types::Value;
use crate::vm::Vm;

fn join(
    vm: &mut Vm,
    _self: Option<HeapRef>,
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
                }))
            }
            Value::HeapRef(r) => {
                let value = vm.resolve_heap_ref(r); 
                match value {
                    HeapObject::String(s) => acc.push(s),
                    _ => {
                        return Err(error::throw(VMErrorType::TypeMismatch {
                            expected: "string".to_string(),
                            received: value.to_string(vm),
                        }))
                    }
                }
        },
            Value::BoundAccess(_) => {
                return Err(error::throw(VMErrorType::TypeMismatch {
                    expected: "string".to_string(),
                    received: "bound_access".to_string(),
                }));
            }
        }
    }
    Ok(Value::HeapRef(vm.heap.allocate(HeapObject::String(
        acc.to_str().unwrap().to_string(),
    ))))
}

pub fn generate_struct() -> (String, Vec<(String, HeapObject)>) {
    (
        "path".to_string(),
        vec![(
            "join".to_string(),
            HeapObject::Function(Function::new(
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
