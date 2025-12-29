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
    fields.push(("slice".to_string(), members::slice_obj()));

    fields
}

pub fn add_handlers(vm: &mut Vm) -> HashMap<String, Value> {
    let mut loaded_members = HashMap::new();

    // if strings lib members are already loaded
    if vm.handlers.contains_key("string.len") {
        if let Some(mem) = vm.get_handler("string.len") {
            loaded_members.insert("len".to_string(), Value::Handle(mem));
        }
        if let Some(mem) = vm.get_handler("string.slice") {
            loaded_members.insert("slice".to_string(), Value::Handle(mem));
        }
    } else {
        let fields = init_lib();
        for (handler_name, handler_obj) in fields {
            let obj_handle = vm.memory.alloc(handler_obj);
            loaded_members.insert(handler_name.clone(), Value::Handle(obj_handle.clone()));

            let handler_name = format!("string.{}", handler_name); // add lib prefix
            vm.handlers.insert(handler_name, obj_handle);
        }
    }

    return loaded_members;
}
