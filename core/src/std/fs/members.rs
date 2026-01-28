use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use crate::core::error::fs_errors::FsError;
use crate::core::error::type_errors::TypeError;
use crate::core::error::{self, VMErrorType};
use crate::memory::Handle;
use crate::std::heap_utils::{put_string, put_vector};
use crate::std::NativeMember;
use crate::types::raw::bool::Bool;
use crate::{
    core::error::VMError,
    memory::MemObject,
    std::ai::members::get_action_context,
    types::{
        object::func::{Engine, Function},
        raw::RawValue,
        Value,
    },
    vm::Vm,
};

fn to_relative(p: &Path) -> PathBuf {
    p.components()
        .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(_)))
        .collect()
}

fn resolve_path_with_context(vm: &Vm, path: &str) -> PathBuf {
    let input = Path::new(path);
    let rel = to_relative(input);

    if let Some(context) = get_action_context(vm) {
        if let Some(cwd_value) = context.get("cwd") {
            if let Ok(cwd_str) = cwd_value.as_string_obj(vm) {
                let cwd_rel = Path::new(&cwd_str);
                let path = cwd_rel.join(rel);
                return path;
            }
        }
    }

    rel
}

// read_file
pub fn read_file_def() -> NativeMember {
    NativeMember {
        name: "read_file".to_string(),
        description: "read a file on the host filesystem on the given path.".to_string(),
        params: Some(vec!["path(string)".to_string()]),
    }
}

pub fn read_file_obj() -> MemObject {
    MemObject::Function(Function::new(
        "read_file".to_string(),
        vec![], // TODO: load params to native functions
        Engine::Native(read_file),
    ))
}

pub fn read_file(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let path = params[0].as_string_obj(vm)?;

    if debug {
        println!("FS.read_file <- {}", path);
    }

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, &path);
    let path_obj = Path::new(&resolved_path);
    if !path_obj.exists() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!(
                "{}",
                resolved_path.display()
            ))),
            vm,
        ));
    }
    if !path_obj.is_file() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::NotAFile(format!("{}", resolved_path.display()))),
            vm,
        ));
    }

    match fs::read(path_obj) {
        Ok(content) => Ok(Value::Handle(put_string(
            vm,
            String::from_utf8_lossy(&content).to_string(),
        ))),
        Err(_) => Err(error::throw(
            VMErrorType::Fs(FsError::ReadError(format!("{}", path))),
            vm,
        )),
    }
}

// read_dir
pub fn read_dir_def() -> NativeMember {
    NativeMember {
        name: "read_dir".to_string(),
        description:
            "read a directory on the host filesystem on the given path and get all the entries."
                .to_string(),
        params: Some(vec!["path(string)".to_string()]),
    }
}

pub fn read_dir_obj() -> MemObject {
    MemObject::Function(Function::new(
        "read_dir".to_string(),
        vec!["path".to_string()], // TODO: load params to native functions
        Engine::Native(read_dir),
    ))
}

pub fn read_dir(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let path = params[0].as_string_obj(vm)?;

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, &path);
    let path_obj = Path::new(&resolved_path);
    if !path_obj.exists() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!(
                "{}",
                resolved_path.display()
            ))),
            vm,
        ));
    }
    if !path_obj.is_dir() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::NotADir(format!("{}", resolved_path.display()))),
            vm,
        ));
    }

    match fs::read_dir(path_obj) {
        Ok(entries) => {
            let mut dir_entries = Vec::new();
            for entry in entries {
                match entry {
                    Ok(dir_entry) => {
                        if let Some(name) = dir_entry.file_name().to_str() {
                            dir_entries.push(Value::Handle(put_string(vm, name.to_string())));
                        }
                    }
                    Err(_) => continue,
                }
            }
            Ok(Value::Handle(put_vector(vm, dir_entries)))
        }
        Err(_) => Err(error::throw(
            VMErrorType::Fs(FsError::ReadError(format!("{}", path))),
            vm,
        )),
    }
}

// write_file
pub fn write_file_def() -> NativeMember {
    NativeMember {
        name: "write_file".to_string(),
        description: "write a file on the host filesystem on the given path. It can also create files depeding on the third flag".to_string(), 
        params: Some(vec![
            "path(string)".to_string(),
            "content(string)".to_string(),
            "create_or_overwrite(bool)".to_string(),
        ])
    }
}

pub fn write_file_obj() -> MemObject {
    MemObject::Function(Function::new(
        "write_file".to_string(),
        vec![
            "path".to_string(),
            "content".to_string(),
            "create_or_overwrite".to_string(),
        ],
        Engine::Native(write_file),
    ))
}

pub fn write_file(
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

    let path = &params[0].as_string_obj(vm)?;
    let content = &params[1].as_string_obj(vm)?;

    if debug {
        println!("FS.write_file <- {}", path);
    }

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, path);
    let path_obj = Path::new(&resolved_path);

    let overwrite_or_create = if let Some(param2) = params.get(2) {
        match param2 {
            Value::RawValue(RawValue::Bool(b)) => b.value,
            _ => {
                return Err(error::throw(
                    VMErrorType::TypeMismatch {
                        expected: "bool".to_string(),
                        received: param2.get_type(),
                    },
                    vm,
                ))
            }
        }
    } else {
        false // default if not passed
    };

    if !path_obj.exists() && !overwrite_or_create {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!(
                "{}",
                resolved_path.display()
            ))),
            vm,
        ));
    }

    let file = if overwrite_or_create {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path_obj)
    } else {
        OpenOptions::new().append(true).open(path_obj)
    };

    match file {
        Ok(mut f) => {
            let write_result = f.write(content.as_bytes());
            match write_result {
                Ok(_) => Ok(Value::RawValue(RawValue::Bool(Bool::new(true)))),
                Err(err) => {
                    println!("err{:#?}", err);
                    Err(error::throw(
                        VMErrorType::Fs(FsError::WriteError(format!(
                            "{}",
                            resolved_path.display()
                        ))),
                        vm,
                    ))
                }
            }
        }
        Err(err) => {
            println!("err{:#?}", err);
            Err(error::throw(
                VMErrorType::Fs(FsError::WriteError(format!("{}", resolved_path.display()))),
                vm,
            ))
        }
    }
}

// delete_file
pub fn delete_def() -> NativeMember {
    NativeMember {
        name: "delete".to_string(), 
        description: "delete a file or a folder on the host filesystem on the given path. The second parameter serves as a flag to delete folders (recursively) or not".to_string(), 
        params: Some(vec![
            "path(string)".to_string(),
            "delete_folder_recursively(string)".to_string(),
        ])
    }
}

pub fn delete_obj() -> MemObject {
    MemObject::Function(Function::new(
        "delete".to_string(),
        vec!["path".to_string()],
        Engine::Native(delete),
    ))
}

pub fn delete(
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

    let path = &params[0].as_string_obj(vm)?;
    let remove_recursively = if let Some(param2) = params.get(1) {
        param2.as_bool(vm)?
    } else {
        false // default if not passed
    };

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, path);
    let path_obj = Path::new(&resolved_path);
    if !path_obj.exists() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!(
                "{}",
                resolved_path.display()
            ))),
            vm,
        ));
    }

    let op_result = if remove_recursively {
        fs::remove_dir_all(path_obj)
    } else {
        fs::remove_file(path_obj)
    };

    match op_result {
        Ok(_) => Ok(Value::RawValue(RawValue::Bool(Bool::new(true)))),
        Err(_) => Err(error::throw(
            VMErrorType::Fs(FsError::DeleteError(format!("{}", resolved_path.display()))),
            vm,
        )),
    }
}

// is_file check a path to see if it's a path is a file
pub fn is_file_def() -> NativeMember {
    NativeMember {
        name: "is_file".to_string(),
        description: "check if the given path is a file (fails when path does not exists)"
            .to_string(),
        params: Some(vec!["path(string)".to_string()]),
    }
}

pub fn is_file_obj() -> MemObject {
    MemObject::Function(Function::new(
        "is_file".to_string(),
        vec!["path".to_string()],
        Engine::Native(is_file),
    ))
}

pub fn is_file(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let path = params[0].as_string_obj(vm)?;

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, &path);
    let path_obj = Path::new(&resolved_path);
    if !path_obj.exists() {
        return Err(error::throw(
            VMErrorType::Fs(FsError::FileNotFound(format!(
                "{}",
                resolved_path.display()
            ))),
            vm,
        ));
    }

    let path_is_file = path_obj.is_file();
    if debug {
        println!(
            "IS_FILE <- {} (resolved to {})",
            path,
            resolved_path.display()
        );
        println!("IS_FILE -> {}", path_is_file);
    }

    if path_is_file {
        return Ok(Value::RawValue(RawValue::Bool(Bool::new(true))));
    } else {
        return Ok(Value::RawValue(RawValue::Bool(Bool::new(false))));
    }
}

// exists check a path to see if it's a path
pub fn exists_def() -> NativeMember {
    NativeMember {
        name: "exists".to_string(),
        description: "check if the given path exists".to_string(),
        params: Some(vec!["path(string)".to_string()]),
    }
}

pub fn exists_obj() -> MemObject {
    MemObject::Function(Function::new(
        "exists".to_string(),
        vec!["path".to_string()],
        Engine::Native(exists),
    ))
}

pub fn exists(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let path = params[0].as_string_obj(vm)?;

    // Resolve path with context
    let resolved_path = resolve_path_with_context(vm, &path);
    let path_obj = Path::new(&resolved_path);
    let path_exists = if path_obj.exists() {
        Bool::new(true)
    } else {
        Bool::new(false)
    };

    if debug {
        println!("FS.EXISTS <- {}({})", path, resolved_path.display());
        println!("EXISTS -> {}", path_exists.value);
    }

    Ok(Value::RawValue(RawValue::Bool(path_exists)))
}
