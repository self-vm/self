use std::collections::HashMap;

use crate::{
    core::error::{self, net_errors::NetErrors, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{buffer::types::Buffer, http::types::HttpResponse, NativeMember},
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
            structs::StructLiteral,
        },
        Value,
    },
    vm::Vm,
};
use futures::future::BoxFuture;
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, ACCEPT, ACCEPT_ENCODING,
        ACCEPT_LANGUAGE, AUTHORIZATION, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE,
        COOKIE, HOST, ORIGIN, REFERER, USER_AGENT,
    },
    Client,
};

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
        let raw_response = client
            .get(url)
            .header(USER_AGENT, "self/0.1")
            .send()
            .await
            .map_err(|e| {
                error::throw(
                    VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                    vm,
                )
            })?;
        let status_code = raw_response.status().as_u16();
        let resp_headers: HashMap<String, String> = raw_response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v.to_str().unwrap_or("").to_string(),
                )
            })
            .collect();
        let raw_body = raw_response.bytes().await.map_err(|e| {
            error::throw(
                VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                vm,
            )
        })?;

        let body_bytes = raw_body.iter().cloned().collect::<Vec<u8>>();
        let body_buffer = Buffer::new_initialized(body_bytes, vm);
        let response = HttpResponse::new_initialized(status_code, resp_headers, body_buffer, vm);
        let response_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::HttpResponse(
                response,
            )));

        Ok(Value::Handle(response_handle))
    })
}

// http.post
pub fn post_obj() -> MemObject {
    MemObject::Function(Function::new(
        "post".to_string(),
        vec!["url".to_string(), "options".to_string()],
        Engine::NativeAsync(post),
    ))
}

pub fn post(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> BoxFuture<'_, Result<Value, VMError>> {
    Box::pin(async move {
        let url = &params[0].as_string_obj(vm)?;
        let options = &params[1].as_struct_obj(vm)?;
        let headers_struct = if let Some(h) = options.property_access("headers") {
            h.as_struct_obj(vm)?
        } else {
            StructLiteral::new("Headers".to_string(), HashMap::new(), vm)
        };
        let body = if let Some(b) = options.property_access("body") {
            b.as_string_obj(vm)?
        } else {
            String::new()
        };

        if debug {
            println!("HTTP.POST -> {}", url);
        }

        let client = Client::new();
        let client_req = client.post(url);

        // set headers
        let mut headers = HeaderMap::new();
        for (header, value) in headers_struct.fields {
            if let Ok(header_name) = parse_header_name(&header) {
                let value = value.as_string_obj(vm)?;
                if let Ok(header_value) = HeaderValue::from_str(&value) {
                    headers.append(header_name, header_value);
                };
            };
        }

        let client = client_req.headers(headers);
        let raw_response = client.body(body).send().await.map_err(|e| {
            error::throw(
                VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                vm,
            )
        })?;
        let status_code = raw_response.status().as_u16();
        let resp_headers: HashMap<String, String> = raw_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let raw_body = raw_response.bytes().await.map_err(|e| {
            error::throw(
                VMErrorType::Net(NetErrors::ReadError(format!("cannot get {}", url))),
                vm,
            )
        })?;

        let body_bytes = raw_body.iter().cloned().collect::<Vec<u8>>();
        let body_buffer = Buffer::new_initialized(body_bytes, vm);
        let response = HttpResponse::new_initialized(status_code, resp_headers, body_buffer, vm);
        let response_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::HttpResponse(
                response,
            )));

        Ok(Value::Handle(response_handle))
    })
}

// UTILS
fn parse_header_name(s: &str) -> Result<HeaderName, InvalidHeaderName> {
    if let Some(_header_name) = header_from_str(s) {
        Ok(_header_name)
    } else {
        HeaderName::from_bytes(s.as_bytes())
    }
}

fn header_from_str(s: &str) -> Option<HeaderName> {
    match s.to_ascii_lowercase().as_str() {
        // Content
        "content-type" => Some(CONTENT_TYPE),
        "content-length" => Some(CONTENT_LENGTH),
        // Auth
        "authorization" => Some(AUTHORIZATION),
        "cookie" => Some(COOKIE),
        // Accept / negotiation
        "accept" => Some(ACCEPT),
        "accept-encoding" => Some(ACCEPT_ENCODING),
        "accept-language" => Some(ACCEPT_LANGUAGE),
        // Caching / connection
        "cache-control" => Some(CACHE_CONTROL),
        "connection" => Some(CONNECTION),
        // Client metadata
        "user-agent" => Some(USER_AGENT),
        "referer" => Some(REFERER),
        "origin" => Some(ORIGIN),
        // Routing
        "host" => Some(HOST),
        _ => None,
    }
}
