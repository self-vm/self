use std::io::{self, Write};

use crate::{
    core::error::VMError,
    memory::{Handle, MemObject},
    std::NativeMember,
    types::{
        object::func::{Engine, Function},
        Value,
    },
    vm::Vm,
};

// read_line
pub fn read_line_def() -> NativeMember {
    NativeMember {
        name: "read_line".to_string(),
        description: "Reads a line from standard input (stdin) and returns it as a string."
            .to_string(),
        params: Some(vec!["".to_string()]),
    }
}

pub fn read_line_obj() -> MemObject {
    MemObject::Function(Function::new(
        "read_line".to_string(),
        vec![],
        Engine::Native(read_line),
    ))
}

pub fn read_line(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // flush the previous stdout
    io::stdout().flush().unwrap();

    // stdin buffer
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim(); // remove the new line

    let stdin_handle = vm.memory.alloc(MemObject::String(input.to_string()));
    Ok(Value::Handle(stdin_handle))
}
