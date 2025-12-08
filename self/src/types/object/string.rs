use std::collections::HashMap;

use crate::{std::selfstring::add_handlers, types::Value, vm::Vm};

#[derive(Debug, Clone)]
pub struct SelfString {
    pub value: String,
    pub members: HashMap<String, Value>,
}

impl SelfString {
    // create new self string with initialized members like: .len() or .split()
    pub fn new(value: String, vm: &mut Vm) -> SelfString {
        let mut self_string = SelfString {
            value,
            members: HashMap::new(),
        };

        if let Some(members_handlers) = add_handlers(vm) {
            self_string.members = members_handlers;
        }

        self_string
    }

    pub fn to_string(&self) -> String {
        self.value.clone()
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        self.members.get(property).cloned()
    }
}
