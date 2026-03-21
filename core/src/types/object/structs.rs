use std::collections::HashMap;

use crate::{
    core::error::{self, fatal_errors::FatalError},
    types::Value,
    vm::Vm,
};
use self_bytecode::DataType;

#[derive(Debug, Clone)]
pub struct StructDeclaration {
    pub identifier: String,
    pub fields: Vec<(String, DataType)>,
}

impl StructDeclaration {
    pub fn new(identifier: String, fields: Vec<(String, DataType)>) -> StructDeclaration {
        StructDeclaration { identifier, fields }
    }
    pub fn to_string(&self) -> String {
        self.identifier.clone()
    }
}

#[derive(Debug, Clone)]
pub struct StructLiteral {
    pub struct_type: String,
    pub fields: HashMap<String, Value>,
    pub members: HashMap<String, Value>,
}

impl StructLiteral {
    pub fn new(struct_type: String, fields: HashMap<String, Value>, vm: &mut Vm) -> StructLiteral {
        let members = crate::std::selfstruct::add_handlers(vm);

        StructLiteral {
            struct_type,
            fields,
            members,
        }
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        if let Some(val) = self.fields.get(property) {
            return Some(val.clone());
        }
        self.members.get(property).cloned()
    }

    pub fn unsafe_property_access(&self, property: &str) -> Value {
        if let Some(field) = self.fields.get(property) {
            field.clone()
        } else {
            error::fatal(FatalError::InvalidPropertyAccess {
                object: self.struct_type.to_string(),
                property: property.to_string(),
            });
        }
    }

    pub fn property_set(&mut self, property: &str, value: Value) {
        self.fields.insert(property.to_string(), value);
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        let fields: Vec<String> = self
            .fields
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.to_string(vm)))
            .collect();
        format!("{}{{ {} }}", self.struct_type, fields.join(", "))
    }
}
