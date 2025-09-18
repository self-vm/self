use crate::{
    std::{
        ai::types::Action,
        mcp::types::{McpClient, McpTool},
        net::types::{NetServer, NetStream},
    },
    types::Value,
};

#[derive(Debug)]
pub enum NativeStruct {
    // net
    NetServer(NetServer),
    NetStream(NetStream),
    // ai
    Action(Action),
    // mcp
    McpClient(McpClient),
    McpTool(McpTool),
}

impl NativeStruct {
    pub fn to_string(&self) -> String {
        match self {
            NativeStruct::NetStream(x) => x.to_string(),
            NativeStruct::NetServer(x) => x.to_string(),
            NativeStruct::Action(x) => x.to_string(),
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
            NativeStruct::McpClient(x) => x.shape.property_access(property),
            NativeStruct::McpTool(x) => x.shape.property_access(property),
        }
    }
}
