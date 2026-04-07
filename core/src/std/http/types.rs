use std::collections::HashMap;

use crate::{
    memory::MemObject,
    std::{ai::utils::sanitize, buffer::types::Buffer},
    types::{
        object::{native_struct::NativeStruct, structs::StructLiteral},
        raw::{u32::U32, utf8::Utf8, RawValue},
        Value,
    },
    vm::Vm,
};

#[derive(Debug)]
pub struct HttpResponse {
    pub shape: StructLiteral,
}

impl HttpResponse {
    pub fn new_initialized(
        status_code: u16,
        headers: HashMap<String, String>,
        body: Buffer,
        vm: &mut Vm,
    ) -> HttpResponse {
        let buf_handle = vm
            .memory
            .alloc(MemObject::NativeStruct(NativeStruct::Buffer(body)));

        // build headers struct and allocate it on the heap
        let mut header_fields = HashMap::new();
        for (key, val) in headers {
            header_fields.insert(key, Value::RawValue(RawValue::Utf8(Utf8::new(val))));
        }
        let headers_struct = StructLiteral::new("Headers".to_string(), header_fields, vm);
        let headers_handle = vm.memory.alloc(MemObject::StructLiteral(headers_struct));

        let mut fields = HashMap::new();
        fields.insert(
            "status_code".to_string(),
            Value::RawValue(RawValue::U32(U32::new(status_code as u32))),
        );
        fields.insert("headers".to_string(), Value::Handle(headers_handle));
        fields.insert("body".to_string(), Value::Handle(buf_handle));

        HttpResponse {
            shape: StructLiteral::new("HttpResponse".to_string(), fields, vm),
        }
    }

    pub fn to_string(&self, vm: &Vm) -> String {
        "HttpResponse {}".to_string()
    }

    pub fn serialize(&self, vm: &Vm, sanitization: bool) -> String {
        let struct_type = self.shape.struct_type.to_string();

        let status_code = self
            .shape
            .unsafe_property_access("status_code")
            .unsafe_as_usize(vm);

        let (headers_str, content_type) = if let MemObject::StructLiteral(s) = vm.memory.resolve(
            &self
                .shape
                .unsafe_property_access("headers")
                .unsafe_as_handle(vm),
        ) {
            let mut content_type = "unknown".to_string();
            if let Some(ct) = s.fields.get("content-type") {
                let raw_content_type = ct.as_string_obj(vm).unwrap_or("unknown".to_string());
                content_type = raw_content_type
                    .split(';')
                    .next()
                    .unwrap_or(&raw_content_type)
                    .to_string();
            }

            (
                s.fields
                    .iter()
                    .map(|(k, v)| format!("    {}: {}", k, v.to_string(vm)))
                    .collect::<Vec<_>>()
                    .join("\n"),
                content_type,
            )
        } else {
            (String::new(), String::new())
        };

        let body_bytes = self
            .shape
            .unsafe_property_access("body")
            .unsafe_as_native_struct(vm)
            .unsafe_as_buffer()
            .bytes
            .clone();

        let body_content = if sanitization {
            sanitize(body_bytes, &content_type)
        } else {
            String::from_utf8_lossy(&body_bytes).to_string()
        };

        // do not send headers for the moment since they
        // tend to be too heavy on production sites
        format!(
            "{}
fields:
  status_code: {}
  body: {}
",
            struct_type, status_code, body_content
        )
    }
}
