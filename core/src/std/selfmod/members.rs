use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::{gen_native_modules_defs, heap_utils::put_string},
    types::{
        object::func::{Engine, Function},
        raw::RawValue,
        Value,
    },
    vm::Vm,
};

pub fn modules_string_obj() -> MemObject {
    MemObject::Function(Function::new(
        "modules_string".to_string(),
        vec![], // TODO: load params to native functions
        Engine::Native(modules_string),
    ))
}

pub fn modules_string(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let stdlib_defs: Vec<String> = gen_native_modules_defs()
        .iter()
        .map(|nm| nm.to_string())
        .collect();
    return Ok(Value::Handle(put_string(
        vm,
        stdlib_defs.join("\n\n----\n"),
    )));
}

pub fn get_stack(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    Ok(Value::RawValue(RawValue::Nothing))
}

pub fn get_stack_fn_ref() -> MemObject {
    MemObject::Function(Function::new(
        "get_stack".to_string(),
        vec![], // TODO: load params to native functions
        Engine::Native(get_stack),
    ))
}
