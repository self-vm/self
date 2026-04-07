use crate::{
    core::error::{self, fatal_errors::FatalError, type_errors, VMError, VMErrorType},
    std::{
        ai::types::{Action, Chain, Context, Link, SessionEnd},
        buffer::types::Buffer,
        http::types::HttpResponse,
        mcp::types::{McpClient, McpTool},
        native::types::NativeLib,
        net::types::{NetServer, NetStream},
        schedule::types::Interval,
        web::types::Browser,
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
    Context(Context),
    Link(Link),
    SessionEnd(SessionEnd),
    // mcp
    McpClient(McpClient),
    McpTool(McpTool),
    // native
    NativeLib(NativeLib),
    // schedule
    Interval(Interval),
    // web
    Browser(Browser),
    // Buffer
    Buffer(Buffer),
    // Http
    HttpResponse(HttpResponse),
}

impl NativeStruct {
    pub fn get_type(&self) -> String {
        match self {
            NativeStruct::NetStream(x) => "NetStream".to_string(),
            NativeStruct::NetServer(x) => "NetStream".to_string(),
            NativeStruct::Action(x) => "NetStream".to_string(),
            NativeStruct::Chain(x) => "Chain".to_string(),
            NativeStruct::Context(x) => "Context".to_string(),
            NativeStruct::Link(x) => "Link".to_string(),
            NativeStruct::SessionEnd(x) => "SessionEnd".to_string(),
            NativeStruct::McpClient(x) => "McpClient".to_string(),
            NativeStruct::McpTool(x) => "McpTool".to_string(),
            NativeStruct::NativeLib(x) => "NativeLib".to_string(),
            NativeStruct::Interval(x) => "Interval".to_string(),
            NativeStruct::Browser(x) => "Browser".to_string(),
            NativeStruct::Buffer(x) => "Buffer".to_string(),
            NativeStruct::HttpResponse(x) => "HttpResponse".to_string(),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        match self {
            NativeStruct::NetStream(x) => x.to_string(),
            NativeStruct::NetServer(x) => x.to_string(),
            NativeStruct::Action(x) => x.to_string(vm),
            NativeStruct::Chain(x) => x.to_string(vm),
            NativeStruct::Context(x) => x.to_string(vm),
            NativeStruct::Link(x) => x.to_string(vm),
            NativeStruct::SessionEnd(x) => x.to_string(),
            NativeStruct::McpClient(x) => x.to_string(),
            NativeStruct::McpTool(x) => x.to_string(),
            NativeStruct::NativeLib(x) => x.to_string(vm),
            NativeStruct::Interval(x) => x.to_string(vm),
            NativeStruct::Browser(x) => x.to_string(vm),
            NativeStruct::Buffer(x) => x.to_string(vm),
            NativeStruct::HttpResponse(x) => x.to_string(vm),
        }
    }

    // this serialization is used when providing this entities
    // to an LLM
    pub fn serialize(&self, vm: &Vm, sanitization: bool) -> String {
        match self {
            NativeStruct::NetStream(x) => x.to_string(),
            NativeStruct::NetServer(x) => x.to_string(),
            NativeStruct::Action(x) => x.to_string(vm),
            NativeStruct::Chain(x) => x.to_string(vm),
            NativeStruct::Context(x) => x.to_string(vm),
            NativeStruct::Link(x) => x.to_string(vm),
            NativeStruct::SessionEnd(x) => x.to_string(),
            NativeStruct::McpClient(x) => x.to_string(),
            NativeStruct::McpTool(x) => x.to_string(),
            NativeStruct::NativeLib(x) => x.to_string(vm),
            NativeStruct::Interval(x) => x.to_string(vm),
            NativeStruct::Browser(x) => x.to_string(vm),
            NativeStruct::Buffer(x) => x.to_string(vm),
            NativeStruct::HttpResponse(x) => x.serialize(vm, sanitization),
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
            NativeStruct::Context(x) => x.property_access(property),
            NativeStruct::Link(x) => x.shape.property_access(property),
            NativeStruct::SessionEnd(x) => x.property_access(property),
            NativeStruct::McpClient(x) => x.shape.property_access(property),
            NativeStruct::McpTool(x) => x.shape.property_access(property),
            NativeStruct::NativeLib(x) => x.property_access(property),
            NativeStruct::Interval(x) => x.property_access(property),
            NativeStruct::Browser(x) => x.property_access(property),
            NativeStruct::Buffer(x) => x.shape.property_access(property),
            NativeStruct::HttpResponse(x) => x.shape.property_access(property),
        }
    }

    // here goes the structs that exposes their internal members
    pub fn get_struct_defs(&self, name: &str) -> Option<String> {
        match self {
            NativeStruct::Browser(x) => Some(x.get_defs(name).to_string()),
            _ => None,
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

    pub fn as_buffer(&self, vm: &Vm) -> Result<&Buffer, VMError> {
        match self {
            NativeStruct::Buffer(x) => Ok(x),
            _ => Err(error::throw(
                VMErrorType::TypeError(type_errors::TypeError::InvalidTypeUnwrap {
                    expected: "Buffer".to_string(),
                    received: self.to_string(vm),
                }),
                vm,
            )),
        }
    }

    // unsafe methods
    pub fn unsafe_as_buffer(&self) -> &Buffer {
        match self {
            NativeStruct::Buffer(x) => x,
            _ => error::fatal(FatalError::InvalidValueUnwrap(self.get_type())),
        }
    }
}
