use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    types::{
        object::func::{Engine, Function},
        raw::{u32::U32, RawValue},
        Value,
    },
    vm::Vm,
};

pub fn len_obj() -> MemObject {
    MemObject::Function(Function::new(
        "len".to_string(),
        vec![],
        Engine::Native(len),
    ))
}

fn len(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // resolve 'self'
    let _self = if let Some(_this) = _self {
        if let MemObject::String(string) = vm.memory.resolve_mut(&_this) {
            string
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    Ok(Value::RawValue(RawValue::U32(U32::new(
        _self.value.len() as u32
    ))))
}
