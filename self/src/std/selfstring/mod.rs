use std::collections::HashMap;

use crate::{
    memory::{Handle, MemObject},
    types::Value,
    vm::Vm,
};
mod members;

pub fn init_lib() -> Vec<(String, MemObject)> {
    let mut fields = vec![];

    fields.push(("len".to_string(), members::len_obj()));

    fields
}

pub fn add_handlers(vm: &mut Vm) -> Option<HashMap<String, Value>> {
    // if strings lib is already loaded
    if vm.handlers.contains_key("string.len") {
        return None;
    }

    let fields = init_lib();
    let mut loaded_members = HashMap::new();
    for (handler_name, handler_obj) in fields {
        let obj_handle = vm.memory.alloc(handler_obj);
        loaded_members.insert(handler_name.clone(), Value::Handle(obj_handle.clone()));

        let handler_name = format!("string.{}", handler_name); // add lib prefix
        vm.handlers.insert(handler_name, obj_handle);
    }

    return Some(loaded_members);
}
