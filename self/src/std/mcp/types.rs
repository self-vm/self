use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;
use rmcp::{model::InitializeRequestParam, service::RunningService, RoleClient};

use crate::{
    memory::MemObject,
    std::mcp::members::{list_tools_obj, shutdown_obj},
    types::{object::structs::StructLiteral, Value},
    vm::Vm,
};

#[derive(Debug)]
pub struct McpClient {
    pub url: String,
    pub client: Arc<Mutex<Option<RunningService<RoleClient, InitializeRequestParam>>>>,
    pub shape: StructLiteral,
}

impl McpClient {
    pub fn new(
        url: String,
        client: RunningService<RoleClient, InitializeRequestParam>,
        shape: StructLiteral,
    ) -> McpClient {
        McpClient {
            url,
            client: Arc::new(Mutex::new(Some(client))),
            shape,
        }
    }

    pub fn initizialize_new(
        url: String,
        client: RunningService<RoleClient, InitializeRequestParam>,
        vm: &mut Vm,
    ) -> McpClient {
        let mut fields = HashMap::new();

        let shutdown_handle = vm.memory.alloc(shutdown_obj());
        let list_tools_handle = vm.memory.alloc(list_tools_obj());

        fields.insert("shutdown".to_string(), Value::Handle(shutdown_handle));
        fields.insert("list_tools".to_string(), Value::Handle(list_tools_handle));

        McpClient {
            url,
            client: Arc::new(Mutex::new(Some(client))),
            shape: StructLiteral::new("McpClient".to_string(), fields),
        }
    }

    pub fn to_string(&self) -> String {
        format!("McpClient({})", self.url)
    }
}

#[derive(Debug)]
pub struct McpTool {
    pub name: String,
    pub shape: StructLiteral,
}

impl McpTool {
    pub fn new(name: String, description: Option<String>, vm: &mut Vm) -> McpTool {
        let mut fields = HashMap::new();

        // add name in struct and in struct.shape to allow
        // direct rust access and direct ego access
        let name_handle = vm.memory.alloc(MemObject::String(name.clone()));
        fields.insert("name".to_string(), Value::Handle(name_handle));

        if let Some(desc) = description {
            let desc_handle = vm.memory.alloc(MemObject::String(desc));
            fields.insert("description".to_string(), Value::Handle(desc_handle));
        }

        McpTool {
            name: name,
            shape: StructLiteral::new("McpTool".to_string(), fields),
        }
    }

    pub fn to_string(&self) -> String {
        format!("McpTool({})", self.name)
    }
}
