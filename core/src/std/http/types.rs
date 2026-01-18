use std::collections::HashMap;

use crate::{
    memory::MemObject,
    std::buffer::types::Buffer,
    types::{
        object::{native_struct::NativeStruct, structs::StructLiteral},
        raw::{u32::U32, RawValue},
        Value,
    },
    vm::Vm,
};

#[derive(Debug)]
pub struct HttpResponse {
    pub shape: StructLiteral,
}

impl HttpResponse {
    pub fn new_initialized(status_code: u16, body: Buffer, vm: &mut Vm) -> HttpResponse {
        let buf_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Buffer(body)));

        let mut fields = HashMap::new();

        fields.insert(
            "status_code".to_string(),
            Value::RawValue(RawValue::U32(U32::new(status_code as u32))),
        );
        fields.insert("body".to_string(), Value::Handle(buf_handle));

        HttpResponse {
            shape: StructLiteral::new("HttpResponse".to_string(), fields),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "HttpResponse {}".to_string()
    }

    pub fn serialize(&self, vm: &Vm) -> String {
        let struct_type = self.shape.struct_type.to_string();
        // status code fallbacks to 0 if it doesn't exists.
        // this should not happen
        let status_code = self
            .shape
            .unsafe_property_access("status_code")
            .unsafe_as_usize(vm);

        // try to serialize body as string always, if it's
        // encoded for another semantics, just put the type
        let body_bytes = self
            .shape
            .unsafe_property_access("body")
            .unsafe_as_native_struct(vm)
            .unsafe_as_buffer()
            .bytes
            .clone();

        format!(
            "{}
fields:
  status_code: {}
  body: {}
",
            struct_type,
            status_code,
            String::from_utf8_lossy(&body_bytes)
        )
    }
}
