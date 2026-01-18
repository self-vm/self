use crate::{
    core::error::{self, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::heap_utils::{put_string, put_vector},
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

pub fn split_obj() -> MemObject {
    MemObject::Function(Function::new(
        "split".to_string(),
        vec!["delimiter".to_string()],
        Engine::Native(split),
    ))
}

fn split(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    _debug: bool,
) -> Result<Value, VMError> {
    if params.is_empty() {
        return Err(error::throw(
            VMErrorType::TypeError(TypeError::InvalidArgsCount {
                expected: 1,
                received: 0,
            }),
            vm,
        ));
    }

    let delimiter = params[0].as_string_obj(vm)?;

    // resolve 'self'
    let self_content = if let Some(_this) = _self {
        if let MemObject::String(string) = vm.memory.resolve(&_this) {
            string.value.clone()
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let mut parts: Vec<Value> = Vec::new();
    for s in self_content.split(&delimiter) {
        let handle = put_string(vm, s.to_string());
        parts.push(Value::Handle(handle));
    }

    let handle = put_vector(vm, parts);
    Ok(Value::Handle(handle))
}
