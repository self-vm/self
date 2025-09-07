use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
};

use rustls::{ClientConnection, StreamOwned};

use crate::{
    types::{object::structs::StructLiteral, Value},
    vm::Vm,
};

#[derive(Debug)]
pub enum StreamKind {
    Plain(TcpStream),
    Tls(StreamOwned<ClientConnection, TcpStream>),
}

use std::io::{Read, Write};

impl Read for StreamKind {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            StreamKind::Plain(s) => s.read(buf),
            StreamKind::Tls(s) => s.read(buf),
        }
    }
}
impl Write for StreamKind {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            StreamKind::Plain(s) => s.write(buf),
            StreamKind::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            StreamKind::Plain(s) => s.flush(),
            StreamKind::Tls(s) => s.flush(),
        }
    }
}

#[derive(Debug)]
pub struct NetServer {
    pub listener: TcpListener,
    pub shape: StructLiteral,
}

impl NetServer {
    pub fn new(listener: TcpListener, shape: HashMap<String, Value>) -> NetServer {
        NetServer {
            listener,
            shape: StructLiteral::new("NetServer".to_string(), shape),
        }
    }

    pub fn to_string(&self) -> String {
        "NetServer".to_string()
    }
}

#[derive(Debug)]
pub struct NetStream {
    pub host: String,
    pub stream: StreamKind,
    pub shape: StructLiteral,
}

impl NetStream {
    pub fn new(host: String, stream: StreamKind, shape: HashMap<String, Value>) -> NetStream {
        NetStream {
            host,
            stream,
            shape: StructLiteral::new("NetStream".to_string(), shape),
        }
    }

    pub fn new_initialized(
        host: String,
        stream: StreamKind,
        shape: HashMap<String, Value>,
        vm: &Vm,
    ) -> NetStream {
        NetStream {
            host,
            stream,
            shape: StructLiteral::new("NetStream".to_string(), shape),
        }
    }

    pub fn to_string(&self) -> String {
        "NetStream".to_string()
    }
}
