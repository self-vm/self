/*
    in order to extend the MCP api read the docs of
    rmcp at https://github.com/modelcontextprotocol/rust-sdk
*/

use crate::{
    core::error::{self, type_errors::TypeError, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::mcp::types::McpClient,
    types::{
        object::{
            func::{Engine, Function},
            native_struct::NativeStruct,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};
use futures::future::BoxFuture;
use rmcp::{
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::StreamableHttpClientTransport,
    ServiceExt,
};

// init an mcp connection
pub fn init_obj() -> MemObject {
    MemObject::Function(Function::new(
        "init".to_string(),
        vec!["url".to_string()],
        Engine::NativeAsync(init),
    ))
}

pub fn init(
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
        if debug {
            println!("MCP_INIT -> {}", url);
        }

        let transport = StreamableHttpClientTransport::from_uri(url.to_string());
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation::default(),
        };

        let client = client_info
            .serve(transport)
            .await
            //.map_err(|e| VMError::new(&format!("mcp: {e}")))?;
            .map_err(|e| error::throw(VMErrorType::ExportInvalidMemberType, vm))?;

        let mcp_client = McpClient::initizialize_new(url.to_string(), client, vm);
        let handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::McpClient(mcp_client)));
        Ok(Value::Handle(handle))
    })
}

// list mcp server tools
pub fn list_tools_obj() -> MemObject {
    MemObject::Function(Function::new(
        "list_tools".to_string(),
        vec![],
        Engine::NativeAsync(list_tools),
    ))
}

pub fn list_tools(
    vm: &mut Vm,
    _self: Option<Handle>,
    _params: Vec<Value>,
    _debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    let client_arc = match _self {
        Some(h) => match vm.memory.resolve(&h) {
            MemObject::NativeStruct(NativeStruct::McpClient(mc)) => mc.client.clone(),
            _ => {
                unreachable!();
            }
        },
        None => unreachable!(),
    };

    Box::pin(async move {
        let guard = client_arc.lock().await;
        let client = guard.as_ref().expect("mcp client it's not connected");

        let tools = client.list_tools(None).await.expect("panic");

        println!("{:#?}", tools);
        Ok(Value::RawValue(RawValue::Nothing))
    })
}

// shutdown mcp connection
pub fn shutdown_obj() -> MemObject {
    MemObject::Function(Function::new(
        "shutdown".to_string(),
        vec![],
        Engine::NativeAsync(shutdown),
    ))
}

pub fn shutdown(
    vm: &mut Vm,
    _self: Option<Handle>,
    _params: Vec<Value>,
    _debug: bool,
) -> BoxFuture<Result<Value, VMError>> {
    let client_arc = match _self {
        Some(h) => match vm.memory.resolve(&h) {
            MemObject::NativeStruct(NativeStruct::McpClient(mc)) => mc.client.clone(),
            _ => {
                unreachable!()
            }
        },
        None => unreachable!(),
    };

    Box::pin(async move {
        let mut guard = client_arc.lock().await;
        if let Some(client) = guard.take() {
            // close
            let _ = client.cancel().await;
        } // already closed
        Ok(Value::RawValue(RawValue::Nothing))
    })
}
