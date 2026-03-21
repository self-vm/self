use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::heap_utils::put_string,
    types::{
        object::func::{Engine, Function},
        Value,
    },
    vm::Vm,
};

// json_object.keys
pub fn keys_obj() -> MemObject {
    MemObject::Function(Function::new(
        "keys".to_string(),
        vec![],
        Engine::Native(keys),
    ))
}

fn keys(
    vm: &mut Vm,
    _self: Option<Handle>,
    _params: Vec<Value>,
    _debug: bool,
) -> Result<Value, VMError> {
    use crate::std::heap_utils::put_vector;
    let self_handle = _self.unwrap();
    let field_keys: Vec<String> =
        if let MemObject::StructLiteral(s) = vm.memory.resolve(&self_handle) {
            s.fields.keys().cloned().collect()
        } else {
            vec![]
        };
    let values: Vec<Value> = field_keys
        .iter()
        .map(|k| Value::Handle(put_string(vm, k.clone())))
        .collect();
    Ok(Value::Handle(put_vector(vm, values)))
}

// json_object.values
pub fn values_obj() -> MemObject {
    MemObject::Function(Function::new(
        "values".to_string(),
        vec![],
        Engine::Native(values),
    ))
}

fn values(
    vm: &mut Vm,
    _self: Option<Handle>,
    _params: Vec<Value>,
    _debug: bool,
) -> Result<Value, VMError> {
    use crate::std::heap_utils::put_vector;
    let self_handle = _self.unwrap();
    let field_values: Vec<Value> =
        if let MemObject::StructLiteral(s) = vm.memory.resolve(&self_handle) {
            s.fields.values().cloned().collect()
        } else {
            vec![]
        };
    Ok(Value::Handle(put_vector(vm, field_values)))
}
