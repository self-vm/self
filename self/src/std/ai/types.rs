use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    memory::{Handle, MemObject},
    std::ai::members::unfold_obj,
    types::{
        object::{
            native_struct::NativeStruct, string::SelfString, structs::StructLiteral, vector::Vector,
        },
        raw::{bool::Bool, utf8::Utf8, RawValue},
        Value,
    },
    vm::Vm,
};

#[derive(Debug, Clone)]
pub struct Action {
    pub module: String,
    pub member: String,
    pub exec: Handle, // handle to the executor function
    pub args: Vec<Value>,
}

impl Action {
    pub fn new(module: String, exec: Handle, member: String, args: Vec<Value>) -> Action {
        Action {
            module,
            exec,
            member,
            args,
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        format!(
            r#"Action({}.{}) {{
  args: [{}]
}}"#,
            self.module,
            self.member,
            self.args
                .iter()
                .map(|v| v.to_string(vm))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        match property {
            "module" => Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                self.module.clone(),
            )))),
            "member" => Some(Value::RawValue(RawValue::Utf8(Utf8::new(
                self.member.clone(),
            )))),
            "exec" => Some(Value::Handle(self.exec.clone())),
            //"params" => self.params,
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub shape: StructLiteral,
    //   `- action: link action
    //   `- def: link one sentence definition
    //   `- is_end: link one sentence definition
    //   `- end_condition: link one sentence definition
    //   `- result: link one sentence definition
}

impl Link {
    pub fn new_initialized(
        link_def: String,
        action: Action,
        is_end: bool,
        end_condition: String,
        result: String,
        vm: &mut Vm,
    ) -> Link {
        let link_def_obj = SelfString::new(link_def, vm);
        let end_condition_obj = SelfString::new(end_condition, vm);
        let result_obj = SelfString::new(result, vm);

        let link_def_handle = vm.memory.alloc(MemObject::String(link_def_obj));
        let end_condition_handle = vm.memory.alloc(MemObject::String(end_condition_obj));
        let result_handle = vm.memory.alloc(MemObject::String(result_obj));
        let action_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Action(action)));

        let mut fields = HashMap::new();
        fields.insert("def".to_string(), Value::Handle(link_def_handle));
        fields.insert("action".to_string(), Value::Handle(action_handle));
        fields.insert(
            "end_condition".to_string(),
            Value::Handle(end_condition_handle),
        );
        fields.insert("result".to_string(), Value::Handle(result_handle));
        fields.insert(
            "is_end".to_string(),
            Value::RawValue(RawValue::Bool(Bool::new(is_end))),
        );

        Link {
            shape: StructLiteral::new("Link".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Link{}".to_string()
    }
}

#[derive(Debug)]
pub struct Chain {
    pub shape: StructLiteral,
}

impl Chain {
    pub fn new_initialized(
        purpose: String,
        end_condition: String,
        chain: Vec<Link>,
        vm: &mut Vm,
    ) -> Chain {
        let purpose_obj = SelfString::new(purpose, vm);
        let end_condition_obj = SelfString::new(end_condition, vm);

        let purpose_handle = vm.memory.alloc(MemObject::String(purpose_obj));
        let end_condition_handle = vm.memory.alloc(MemObject::String(end_condition_obj));
        let unfold_handle = vm.memory.alloc(unfold_obj());

        // chain
        let mut handles_chain = vec![];
        for link in chain.iter() {
            let link_handle = vm
                .memory
                .alloc(MemObject::NativeStruct(NativeStruct::Link(link.clone())));
            handles_chain.push(Value::Handle(link_handle));
        }
        let links_handle = vm.memory.alloc(MemObject::Vector(Vector::new_initialized(
            handles_chain,
            vm,
        )));

        // populate fields
        let mut fields = HashMap::new();
        fields.insert("purpose".to_string(), Value::Handle(purpose_handle));
        fields.insert(
            "end_condition".to_string(),
            Value::Handle(end_condition_handle),
        );
        fields.insert("links".to_string(), Value::Handle(links_handle));
        fields.insert("unfold".to_string(), Value::Handle(unfold_handle));

        Chain {
            shape: StructLiteral::new("Chain".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Chain{}".to_string()
    }
}

pub struct UnfoldStore {
    pub prev_links: Vec<String>,
    pub context: Vec<Value>,
}

impl UnfoldStore {
    pub fn new() -> UnfoldStore {
        UnfoldStore {
            prev_links: vec![],
            context: vec![],
        }
    }
}

// AI json serdes types
#[derive(Debug, Default, Deserialize, Clone)]
pub struct AIAction {
    pub module: String,
    pub member: String,
    pub params: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChainLinkJson {
    #[serde(default)]
    pub link_def: String,
    #[serde(default)]
    pub link_action: AIAction,
    #[serde(default)]
    pub end: bool,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub end_condition: String,
}
