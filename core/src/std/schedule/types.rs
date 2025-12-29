use std::collections::HashMap;

use crate::{
    std::schedule::members::start_obj,
    types::{object::structs::StructLiteral, Value},
    vm::Vm,
};

#[derive(Debug, Clone)]
pub struct Interval {
    pub shape: StructLiteral,
    //   `- action: link action
    //   `- def: link one sentence definition
    //   `- is_end: link one sentence definition
    //   `- end_condition: link one sentence definition
    //   `- result: link one sentence definition
}

impl Interval {
    pub fn new_initialized(vm: &mut Vm) -> Interval {
        let start_handle = vm.memory.alloc(start_obj());

        let mut fields = HashMap::new();
        fields.insert("start".to_string(), Value::Handle(start_handle));

        Interval {
            shape: StructLiteral::new("Interval".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Interval {}".to_string()
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        self.shape.property_access(property)
    }
}
