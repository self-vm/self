use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::heap_utils::put_string,
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
        if let MemObject::String(string) = vm.memory.resolve(&_this) {
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

pub fn slice_obj() -> MemObject {
    MemObject::Function(Function::new(
        "slice".to_string(),
        vec![],
        Engine::Native(slice),
    ))
}

fn slice(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // resolve 'self'
    let _self = if let Some(_this) = _self {
        if let MemObject::String(string) = vm.memory.resolve(&_this) {
            string
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let start = params[0].as_usize(vm)?;
    let mut end = params[1].as_usize(vm)?;

    if end > _self.value.len() {
        end = _self.value.len() - 1;
    }

    let new_string = &_self.value[start..end];
    let handle = put_string(vm, new_string.to_string());
    Ok(Value::Handle(handle))
}
