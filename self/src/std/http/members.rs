/*
    in order to extend the MCP api read the docs of
    rmcp at https://github.com/modelcontextprotocol/rust-sdk
*/

use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{
        heap_utils::put_string,
        mcp::types::{McpClient, McpTool},
        NativeMember,
    },
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            vector::Vector,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};
use futures::future::BoxFuture;
use reqwest::Client;

// http.get
pub fn get_obj() -> MemObject {
    MemObject::Function(Function::new(
        "get".to_string(),
        vec!["url".to_string()],
        Engine::NativeAsync(get),
    ))
}

pub fn get_def() -> NativeMember {
    NativeMember {
        name: "get".to_string(),
        description: "Http GET request to the given url.".to_string(),
        params: Some(vec!["url(string)".to_string()]),
    }
}

pub fn get(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        if params.len() < 1 {
            return Err(error::throw(
                VMErrorType::TypeError(TypeError::InvalidArgsCount {
                    expected: 1,
                    received: params.len() as u32,
                }),
                vm,
            ));
        }

        let url = &params[0].as_string_obj(vm)?;
        //let config = &params[1].as_struct_obj(vm)?;

        if debug {
            println!("HTTP.GET -> {}", url);
        }

        let client = Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                error::throw(
                    VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                    vm,
                )
            })?
            .text()
            .await
            .map_err(|e| {
                error::throw(
                    VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                    vm,
                )
            })?;

        let handle = put_string(vm, response);
        Ok(Value::Handle(handle))
    })
}
