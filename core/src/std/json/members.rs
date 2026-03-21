use std::collections::HashMap;

use crate::{
    core::error::{self, json_errors::JsonErrors, VMError, VMErrorType},
    memory::{Handle, MemObject},
    std::{heap_utils::put_string, NativeMember},
    types::{
        object::{
            func::{Engine, Function},
            structs::StructLiteral,
            vector::Vector,
        },
        raw::RawValue,
        Value,
    },
    vm::Vm,
};
use serde_json::{json, Map, Value as JsonValue};

// json.encode
pub fn encode_obj() -> MemObject {
    MemObject::Function(Function::new(
        "encode".to_string(),
        vec!["value".to_string()],
        Engine::Native(encode),
    ))
}

pub fn encode_def() -> NativeMember {
    NativeMember {
        name: "encode".to_string(),
        description: "Encodes a value (struct, vector, or primitive) to a JSON string.".to_string(),
        params: Some(vec!["value(any)".to_string()]),
    }
}

pub fn encode(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let value = &params[0];

    if debug {
        println!("JSON.ENCODE -> {:?}", value);
    };

    let json_value = value_to_json(vm, value)?;
    let json_string = serde_json::to_string(&json_value).map_err(|e| {
        error::throw(
            VMErrorType::TypeMismatch {
                expected: "serializable value".to_string(),
                received: format!("JSON serialization error: {}", e),
            },
            vm,
        )
    })?;

    let handle = put_string(vm, json_string);
    Ok(Value::Handle(handle))
}

/// converts a self value to a serde_json::Value
fn value_to_json(vm: &Vm, value: &Value) -> Result<JsonValue, VMError> {
    match value {
        Value::RawValue(raw) => raw_value_to_json(raw),
        Value::Handle(handle) => {
            let mem_obj = vm.memory.resolve(handle);
            mem_object_to_json(vm, mem_obj)
        }
        Value::BoundAccess(bound) => {
            // Resolve the bound access value
            value_to_json(vm, &bound.property)
        }
    }
}

fn raw_value_to_json(raw: &RawValue) -> Result<JsonValue, VMError> {
    Ok(match raw {
        RawValue::I32(x) => json!(x.value),
        RawValue::I64(x) => json!(x.value),
        RawValue::U32(x) => json!(x.value),
        RawValue::U64(x) => json!(x.value),
        RawValue::F64(x) => json!(x.value),
        RawValue::Utf8(x) => json!(x.value),
        RawValue::Bool(x) => json!(x.value),
        RawValue::Byte(x) => json!(x.value),
        RawValue::Nothing => JsonValue::Null,
    })
}

fn mem_object_to_json(vm: &Vm, mem_obj: &MemObject) -> Result<JsonValue, VMError> {
    match mem_obj {
        MemObject::String(s) => Ok(json!(s.value)),
        MemObject::StructLiteral(s) => struct_to_json(vm, s),
        MemObject::Vector(v) => vector_to_json(vm, v),
        _ => Err(error::throw(
            VMErrorType::Json(JsonErrors::EncodingError(mem_obj.get_type())),
            vm,
        )),
    }
}

fn struct_to_json(vm: &Vm, s: &StructLiteral) -> Result<JsonValue, VMError> {
    let mut map = Map::new();

    for (key, value) in &s.fields {
        let json_value = value_to_json(vm, value)?;
        map.insert(key.clone(), json_value);
    }

    Ok(JsonValue::Object(map))
}

fn vector_to_json(vm: &Vm, v: &Vector) -> Result<JsonValue, VMError> {
    let mut arr = Vec::new();

    for element in &v.elements {
        let json_value = value_to_json(vm, element)?;
        arr.push(json_value);
    }

    Ok(JsonValue::Array(arr))
}

// json.decode
pub fn decode_obj() -> MemObject {
    MemObject::Function(Function::new(
        "decode".to_string(),
        vec!["json_string".to_string()],
        Engine::Native(decode),
    ))
}

pub fn decode_def() -> NativeMember {
    NativeMember {
        name: "decode".to_string(),
        description: "Decodes a JSON string into a value (struct, vector, or primitive)."
            .to_string(),
        params: Some(vec!["json_string(string)".to_string()]),
    }
}

pub fn decode(
    vm: &mut Vm,
    _self: Option<Handle>,
    params: Vec<Value>,
    debug: bool,
) -> Result<Value, VMError> {
    let json_str = params[0].as_string_obj(vm)?;

    if debug {
        println!("JSON.DECODE -> {}", json_str);
    };

    let json_value: JsonValue = serde_json::from_str(&json_str).map_err(|e| {
        error::throw(
            VMErrorType::TypeMismatch {
                expected: "valid JSON string".to_string(),
                received: format!("JSON parse error: {}", e),
            },
            vm,
        )
    })?;

    json_to_value(vm, &json_value)
}

/// Converts a serde_json::Value to a self value
fn json_to_value(vm: &mut Vm, json: &JsonValue) -> Result<Value, VMError> {
    use crate::types::raw::{bool::Bool, f64::F64, i64::I64};

    match json {
        JsonValue::Null => Ok(Value::RawValue(RawValue::Nothing)),
        JsonValue::Bool(b) => Ok(Value::RawValue(RawValue::Bool(Bool::new(*b)))),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::RawValue(RawValue::I64(I64::new(i))))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::RawValue(RawValue::F64(F64::new(f))))
            } else {
                // Fallback for very large numbers
                Ok(Value::RawValue(RawValue::F64(F64::new(
                    n.as_f64().unwrap_or(0.0),
                ))))
            }
        }
        JsonValue::String(s) => {
            let handle = put_string(vm, s.clone());
            Ok(Value::Handle(handle))
        }
        JsonValue::Array(arr) => {
            let mut elements = Vec::new();
            for item in arr {
                let value = json_to_value(vm, item)?;
                elements.push(value);
            }
            let vector = Vector::new_initialized(elements, vm);
            let handle = vm.memory.alloc(MemObject::Vector(vector));
            Ok(Value::Handle(handle))
        }
        JsonValue::Object(obj) => {
            let mut fields = HashMap::new();
            for (key, val) in obj {
                let value = json_to_value(vm, val)?;
                fields.insert(key.clone(), value);
            }
            let struct_literal = StructLiteral::new("json_object".to_string(), fields, vm);
            let handle = vm.memory.alloc(MemObject::StructLiteral(struct_literal));
            Ok(Value::Handle(handle))
        }
    }
}
