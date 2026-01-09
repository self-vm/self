use std::collections::HashMap;

use crate::{types::object::structs::StructLiteral, vm::Vm};

// this struct can be used as constructor
// or instance. both are valid
#[derive(Debug)]
pub struct Buffer {
    pub bytes: Vec<u8>,
    pub shape: StructLiteral,
}

impl Buffer {
    pub fn new_initialized(bytes: Vec<u8>, vm: &mut Vm) -> Buffer {
        let fields = HashMap::new();
        let shape = StructLiteral::new("Buffer".to_string(), fields);

        Buffer { bytes, shape }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "Buffer {}".to_string()
    }
}
