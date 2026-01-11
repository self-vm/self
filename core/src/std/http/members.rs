use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{
        buffer::types::Buffer,
        heap_utils::put_string,
        http::types::HttpResponse,
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
        let raw_response = client.get(url).send().await.map_err(|e| {
            error::throw(
                VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                vm,
            )
        })?;
        let status_code = raw_response.status().as_u16();
        let raw_body = raw_response.bytes().await.map_err(|e| {
            error::throw(
                VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                vm,
            )
        })?;

        let body_bytes = raw_body.iter().map(|v| v.clone()).collect::<Vec<u8>>();
        let body_buffer = Buffer::new_initialized(body_bytes, vm);
        let response = HttpResponse::new_initialized(status_code, body_buffer, vm);
        let response_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::HttpResponse(
                response,
            )));

        Ok(Value::Handle(response_handle))
    })
}
