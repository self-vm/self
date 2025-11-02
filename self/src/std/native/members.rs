use core::ffi::c_str;
use std::ffi::{c_char, CString};
use std::path::Path;

use crate::core::error::fs_errors::FsError;
use crate::core::error::type_errors::TypeError;
use crate::core::error::{self, VMErrorType};
use crate::memory::Handle;
use crate::std::native::types::NativeLib;
use crate::std::NativeMember;
use crate::types::object::native_struct::NativeStruct;
use crate::{
    core::error::VMError,
    memory::MemObject,
    types::{
        object::func::{Engine, Function},
        raw::RawValue,
        Value,
    },
    vm::Vm,
};
use libloading::{Library, Symbol};

// load_lib
pub fn load_lib_def() -> NativeMember {
    NativeMember {
        name: "load_lib".to_string(),
        description: "load a shared library (.so, .dll) to the self-vm context. works as ffi."
            .to_string(),
        params: Some(vec!["path(string)".to_string()]),
    }
}

pub fn load_lib_obj() -> MemObject {
    MemObject::Function(Function::new(
        "load_lib".to_string(),
        vec!["path".to_string()], // TODO: load params to native functions
        Engine::Native(load_lib),
    ))
}

pub fn load_lib(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let path = params[0].as_string_obj(vm)?;
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!("{}", path))),
            vm,
        ));
    }
    if !path_obj.is_file() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::NotAFile(format!("{}", path))),
            vm,
        ));
    }

    let lib = unsafe {
        Library::new(path_obj).map_err(|e| {
            error::throw(
                VMErrorType::Fs(FsError::ReadError(format!("{}: {}", path, e))),
                vm,
            )
        })
    }?;

    let natlib = NativeLib::new_initialized(path, lib, vm);
    let natlib_handle = vm
        .memory
        .alloc(MemObject::NativeStruct(NativeStruct::NativeLib(natlib)));

    Ok(Value::Handle(natlib_handle))
}

pub fn call(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // TODO: here we should implement a generic function
    // for typechecking the self value resolve
    let (_self, _self_ref) = if let Some(_this) = _self {
        if let MemObject::NativeStruct(NativeStruct::NativeLib(v)) = vm.memory.resolve(&_this) {
            (v, _this)
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    if params.len() < 1 {
        return Err(error::throw(
            VMErrorType::TypeError(TypeError::InvalidArgsCount {
                expected: 1,
                received: params.len() as u32,
            }),
            vm,
        ));
    }

    // get function symbol from lib
    let function_name = params[0].as_string_obj(vm)?;
    let function_arg = params[1].as_string_obj(vm)?;

    let func: Symbol<unsafe extern "C" fn(*const c_char)> = unsafe {
        _self
            .library
            .get(format!("{}\0", function_name).as_bytes())
            .map_err(|e| {
                error::throw(
                    VMErrorType::Fs(FsError::ReadError(format!(
                        "{}: no symbol {}: {}",
                        _self.path, function_name, e
                    ))),
                    vm,
                )
            })
    }?;

    let name = CString::new(function_arg).map_err(|e| {
        error::throw(
            VMErrorType::Fs(FsError::ReadError(format!("bad cstr: {}", e))),
            vm,
        )
    })?;

    unsafe {
        func(name.as_ptr());
    };

    if debug {
        // todo:
    }

    Ok(Value::RawValue(RawValue::Nothing))
}
