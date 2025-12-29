use crate::{
    core::error::{self, fs_errors::FsError, type_errors::TypeError, VMError, VMErrorType},
    heap::HeapRef,
    memory::{Handle, MemObject},
    types::{
        object::{
            func::{Engine, Function},
            string::SelfString,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};
use std::{env, path::PathBuf};

// environment variable set
pub fn set_obj() -> MemObject {
    MemObject::Function(Function::new(
        "set".to_string(),
        vec!["key".to_string(), "value".to_string()],
        Engine::Native(set),
    ))
}

pub fn set(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    if params.len() < 2 {
        return Err(error::throw(
            VMErrorType::TypeError(TypeError::InvalidArgsCount {
                expected: 2,
                received: params.len() as u32,
            }),
            vm,
        ));
    }

    let key = &params[0].as_string_obj(vm)?;
    let value = &params[1].as_string_obj(vm)?;

    if debug {
        println!("ENV_SET -> {}({})", key, value)
    }
    env::set_var(key, value);
    Ok(Value::RawValue(RawValue::Nothing))
}

// get environment variables
pub fn get_obj() -> MemObject {
    MemObject::Function(Function::new(
        "get".to_string(),
        vec!["key".to_string()],
        Engine::Native(get),
    ))
}

pub fn get(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    if params.len() < 1 {
        return Err(error::throw(
            VMErrorType::TypeError(TypeError::InvalidArgsCount {
                expected: 1,
                received: params.len() as u32,
            }),
            vm,
        ));
    }

    let key = &params[0].as_string_obj(vm)?;

    if debug {
        println!("ENV_GET -> {}", key)
    }
    let var = env::var(key);
    match var {
        Ok(v) => {
            let string_obj = SelfString::new(v, vm);
            let handle = vm.memory.alloc(MemObject::String(string_obj));
            Ok(Value::Handle(handle))
        }
        Err(_) => Ok(Value::RawValue(RawValue::Nothing)),
    }
}

// read a .env file
pub fn read_obj() -> MemObject {
    MemObject::Function(Function::new(
        "read".to_string(),
        vec![],
        Engine::Native(read),
    ))
}

pub fn read(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let cwd = env::current_dir().map_err(|e| {
        // TODO: use self-vm errors system
        panic!("cannot get cwd in env.read");
    })?;

    let path_buf = if let Some(v) = params.get(0) {
        let s = v.as_string_obj(vm)?;
        let mut p = PathBuf::from(&s);

        if p.is_relative() {
            p = cwd.join(p);
        }

        if p.is_dir() {
            p = p.join(".env");
        }

        p
    } else {
        cwd.join(".env")
    };

    if debug {
        println!("ENV_READ -> {}", path_buf.display());
    }

    let p = path_buf.display().to_string();
    dotenvy::from_path(path_buf).map_err(|e| match e {
        dotenvy::Error::Io(ioe) if ioe.kind() == std::io::ErrorKind::NotFound => {
            error::throw(VMErrorType::Fs(FsError::FileNotFound(p)), vm)
        }
        _ => error::throw(VMErrorType::Fs(FsError::FileNotFound(p)), vm),
    })?;

    Ok(Value::RawValue(RawValue::Nothing))
}
