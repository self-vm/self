use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{
        buffer::types::Buffer,
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
            .bytes()
            .await
            .map_err(|e| {
                error::throw(
                    VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                    vm,
                )
            })?;

        let bytes = response.iter().map(|v| v.clone()).collect::<Vec<u8>>();
        let buffer = Buffer::new_initialized(bytes, vm);
        let buf_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Buffer(buffer)));
        Ok(Value::Handle(buf_handle))
    })
}
