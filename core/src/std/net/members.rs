use core::panic;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::core::error::net_errors::NetErrors;
use crate::core::error::{self, VMErrorType};
use crate::memory::Handle;
use crate::std::heap_utils::put_string;
use crate::std::net::types::{NetServer, NetStream, StreamKind};
use crate::std::net::utils::tls;
use crate::types::object::native_struct::NativeStruct;
use crate::types::raw::u64::U64;
use crate::types::raw::RawValue;
use crate::{
    core::error::VMError,
    memory::MemObject,
    types::{
        object::func::{Engine, Function},
        Value,
    },
    vm::Vm,
};

// connect
pub fn connect_ref() -> MemObject {
    MemObject::Function(Function::new(
        "connect".to_string(),
        vec!["host".to_string()],
        Engine::Native(connect),
    ))
}

pub fn connect(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let host = params[0].as_string_obj(vm)?;
    let use_tls = if let Some(second) = params.get(1) {
        second.as_bool(vm)?
    } else {
        false // default if not passed
    };

    let stream = if use_tls {
        let tls_stream = tls(&host);
        if let Ok(_stream) = tls_stream {
            StreamKind::Tls(_stream)
        } else {
            return Err(error::throw(
                VMErrorType::Net(NetErrors::NetConnectError(format!("host {}", host))),
                vm,
            ));
        }
    } else {
        if let Ok(stream) = TcpStream::connect(host.clone()) {
            StreamKind::Plain(stream)
        } else {
            return Err(error::throw(
                VMErrorType::Net(NetErrors::NetConnectError(format!("host {}", host))),
                vm,
            ));
        }
    };

    let mut shape = HashMap::new();
    let owned_host = host.clone();
    let host_ref = put_string(vm, host.clone());
    let write_ref = vm.memory.alloc(MemObject::Function(Function::new(
        "write".to_string(),
        vec![],
        Engine::Native(write),
    )));
    let read_ref = vm.memory.alloc(MemObject::Function(Function::new(
        "read".to_string(),
        vec![],
        Engine::Native(read),
    )));

    shape.insert("host".to_string(), Value::Handle(host_ref));
    shape.insert("write".to_string(), Value::Handle(write_ref));
    shape.insert("read".to_string(), Value::Handle(read_ref));

    let net_stream = NetStream::new(owned_host, stream, shape);
    let net_stream_ref = vm
        .memory
        .alloc(MemObject::NativeStruct(NativeStruct::NetStream(net_stream)));

    return Ok(Value::Handle(net_stream_ref));
}

fn write(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // get params
    let data = params[0].as_string_obj(vm)?;
    // resolve 'self'
    let _self = if let Some(_this) = _self {
        if let MemObject::NativeStruct(NativeStruct::NetStream(ns)) = vm.memory.resolve_mut(&_this)
        {
            ns
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let write_result = _self.stream.write(data.as_bytes());
    if let Ok(bytes) = write_result {
        Ok(Value::RawValue(RawValue::U64(U64::new(bytes as u64))))
    } else {
        Err(error::throw(
            VMErrorType::Net(NetErrors::WriteError(_self.host.to_string())),
            vm,
        ))
    }
}

fn read(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // resolve 'self'
    let _self = if let Some(_this) = _self {
        if let MemObject::NativeStruct(NativeStruct::NetStream(ns)) = vm.memory.resolve_mut(&_this)
        {
            ns
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let mut buffer = [0; 4096];
    let read_result = _self.stream.read(&mut buffer);
    let bytes_count = if let Ok(bytes_count) = read_result {
        bytes_count
    } else {
        return Err(error::throw(
            VMErrorType::Net(NetErrors::ReadError(_self.host.to_string())),
            vm,
        ));
    };
    Ok(Value::Handle(put_string(
        vm,
        String::from_utf8_lossy(&buffer[..bytes_count]).to_string(),
    )))
}

///// listen
pub fn listen_ref() -> MemObject {
    MemObject::Function(Function::new(
        "listen".to_string(),
        vec!["port".to_string()],
        Engine::Native(listen),
    ))
}

pub fn listen(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let port = params[0].as_string_obj(vm)?;
    let host = format!("127.0.0.1:{}", port);
    let server = match TcpListener::bind(host.clone()) {
        Ok(v) => v,
        Err(err) => panic!("cannot listen on the provided port"),
    };

    let mut shape = HashMap::new();
    let accept_ref = vm.memory.alloc(MemObject::Function(Function::new(
        "accept".to_string(),
        vec![],
        Engine::Native(accept),
    )));
    shape.insert("accept".to_string(), Value::Handle(accept_ref));

    let net_server = NetServer::new(server, shape);
    return Ok(Value::Handle(vm.memory.alloc(MemObject::NativeStruct(
        NativeStruct::NetServer(net_server),
    ))));
}

fn accept(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    // resolve 'self'
    let _self = if let Some(_this) = _self {
        if let MemObject::NativeStruct(NativeStruct::NetServer(ns)) = vm.memory.resolve_mut(&_this)
        {
            ns
        } else {
            unreachable!()
        }
    } else {
        unreachable!()
    };

    let (stream, sock_addr) = match _self.listener.accept() {
        Ok(v) => v,
        Err(err) => {
            panic!("test")
        }
    };

    let host = sock_addr.to_string();
    let mut shape = HashMap::new();
    let host_ref = put_string(vm, host.clone());
    let write_ref = vm.memory.alloc(MemObject::Function(Function::new(
        "write".to_string(),
        vec![],
        Engine::Native(write),
    )));
    let read_ref = vm.memory.alloc(MemObject::Function(Function::new(
        "read".to_string(),
        vec![],
        Engine::Native(read),
    )));
    shape.insert("host".to_string(), Value::Handle(host_ref));
    shape.insert("write".to_string(), Value::Handle(write_ref));
    shape.insert("read".to_string(), Value::Handle(read_ref));
    let net_stream = NetStream::new(host, StreamKind::Plain(stream), shape);
    let net_stream_ref = vm
        .memory
        .alloc(MemObject::NativeStruct(NativeStruct::NetStream(net_stream)));
    return Ok(Value::Handle(net_stream_ref));
}
