use crate::{
    core::error::{self, type_errors, VMError, VMErrorType},
    std::{
        ai::types::{Action, Chain, Link},
        mcp::types::{McpClient, McpTool},
        net::types::{NetServer, NetStream},
    },
    types::Value,
    vm::Vm,
};

#[derive(Debug)]
pub enum NativeStruct {
    // net
    NetServer(NetServer),
    NetStream(NetStream),
    // ai
    Action(Action),
    Chain(Chain),
    Link(Link),
    // mcp
    McpClient(McpClient),
    McpTool(McpTool),
}

impl NativeStruct {
    pub fn to_string(&self, vm: &Vm) -> String {
        match self {
            NativeStruct::NetStream(x) => x.to_string(),
            NativeStruct::NetServer(x) => x.to_string(),
            NativeStruct::Action(x) => x.to_string(vm),
            NativeStruct::Chain(x) => x.to_string(vm),
            NativeStruct::Link(x) => x.to_string(vm),
            NativeStruct::McpClient(x) => x.to_string(),
            NativeStruct::McpTool(x) => x.to_string(),
        }
    }

    pub fn property_access(&self, property: &str) -> Option<Value> {
        // here the property accesses values are owned. we're
        // bringing or the ref to the value or the value
        // itself
        match self {
            NativeStruct::NetStream(x) => x.shape.property_access(property),
            NativeStruct::NetServer(x) => x.shape.property_access(property),
            NativeStruct::Action(x) => x.property_access(property),
            NativeStruct::Chain(x) => x.shape.property_access(property),
            NativeStruct::Link(x) => x.shape.property_access(property),
            NativeStruct::McpClient(x) => x.shape.property_access(property),
            NativeStruct::McpTool(x) => x.shape.property_access(property),
        }
    }

    pub fn as_link(&self, vm: &Vm) -> Result<Link, VMError> {
        match self {
            NativeStruct::Link(x) => Ok(x.clone()),
            _ => Err(error::throw(
                VMErrorType::TypeError(type_errors::TypeError::InvalidTypeUnwrap {
                    expected: "Link".to_string(),
                    received: self.to_string(vm),
                }),
                vm,
            )),
        }
    }

    pub fn as_action(&self, vm: &Vm) -> Result<Action, VMError> {
        match self {
            NativeStruct::Action(x) => Ok(x.clone()),
            _ => Err(error::throw(
                VMErrorType::TypeError(type_errors::TypeError::InvalidTypeUnwrap {
                    expected: "Link".to_string(),
                    received: self.to_string(vm),
                }),
                vm,
            )),
        }
    }
}
