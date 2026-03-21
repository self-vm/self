use std::collections::HashMap;

use crate::{
    memory::MemObject,
    std::selfstruct::members::{keys_obj, values_obj},
    types::Value,
    vm::Vm,
};

mod members;

pub fn init_lib() -> Vec<(String, MemObject)> {
    vec![
        ("keys".to_string(), keys_obj()),
        ("values".to_string(), values_obj()),
    ]
}

pub fn add_handlers(vm: &mut Vm) -> HashMap<String, Value> {
    let mut loaded_members = HashMap::new();

    if vm.handlers.contains_key("struct_literal.keys") {
        if let Some(mem) = vm.get_handler("struct_literal.keys") {
            loaded_members.insert("keys".to_string(), Value::Handle(mem));
        }
        if let Some(mem) = vm.get_handler("struct_literal.values") {
            loaded_members.insert("values".to_string(), Value::Handle(mem));
        }
    } else {
        let fields = init_lib();
        for (handler_name, handler_obj) in fields {
            let obj_handle = vm.memory.alloc(handler_obj);
            loaded_members.insert(handler_name.clone(), Value::Handle(obj_handle.clone()));
            let handler_name = format!("struct_literal.{}", handler_name);
            vm.handlers.insert(handler_name, obj_handle);
        }
    }

    loaded_members
}
