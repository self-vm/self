pub mod bool;
pub mod byte;
pub mod f64;
pub mod i32;
pub mod i64;
pub mod u32;
pub mod u64;
pub mod utf8;

use bool::Bool;
use f64::F64;
use i32::I32;
use i64::I64;
use u32::U32;
use u64::U64;
use utf8::Utf8;

use crate::{
    core::error::{self, VMError, VMErrorType},
    std::ai::utils::{is_sanitizable, sanitize},
    types::raw::byte::Byte,
    vm::Vm,
};
use self_bytecode::DataType;

#[derive(Debug, Clone)]
pub enum RawValue {
    I32(I32),
    I64(I64),
    U32(U32),
    U64(U64),
    F64(F64),
    Utf8(Utf8),
    Bool(Bool),
    Byte(Byte),
    Nothing,
}

impl RawValue {
    pub fn get_type(&self) -> DataType {
        match self {
            RawValue::I32(_) => DataType::I32,
            RawValue::I64(_) => DataType::I64,
            RawValue::U32(_) => DataType::U32,
            RawValue::U64(_) => DataType::U64,
            RawValue::F64(_) => DataType::F64,
            RawValue::Utf8(_) => DataType::Utf8,
            RawValue::Bool(_) => DataType::Bool,
            RawValue::Byte(_) => DataType::Byte,
            RawValue::Nothing => DataType::Nothing,
        }
    }

    pub fn get_type_string(&self) -> String {
        match self {
            RawValue::I32(_) => "I32".to_string(),
            RawValue::I64(_) => "I64".to_string(),
            RawValue::U32(_) => "U32".to_string(),
            RawValue::U64(_) => "U64".to_string(),
            RawValue::F64(_) => "F64".to_string(),
            RawValue::Utf8(_) => "UTF8".to_string(),
            RawValue::Bool(_) => "BOOL".to_string(),
            RawValue::Byte(_) => "BYTE".to_string(),
            RawValue::Nothing => "NOTHING".to_string(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            RawValue::I32(x) => x.value.to_string(),
            RawValue::I64(x) => x.value.to_string(),
            RawValue::U32(x) => x.value.to_string(),
            RawValue::U64(x) => x.value.to_string(),
            RawValue::F64(x) => x.value.to_string(),
            RawValue::Utf8(x) => x.value.to_string(),
            RawValue::Bool(x) => x.value.to_string(),
            RawValue::Byte(x) => x.value.to_string(),
            RawValue::Nothing => "nothing".to_string(),
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            RawValue::I32(x) => x.value.to_string(),
            RawValue::I64(x) => x.value.to_string(),
            RawValue::U32(x) => x.value.to_string(),
            RawValue::U64(x) => x.value.to_string(),
            RawValue::F64(x) => x.value.to_string(),
            RawValue::Utf8(x) => {
                if let Some(content_type) = is_sanitizable(&x.value) {
                    sanitize(x.value.as_bytes().to_vec(), content_type)
                } else {
                    x.value.to_string()
                }
            }
            RawValue::Bool(x) => x.value.to_string(),
            RawValue::Byte(x) => x.value.to_string(),
            RawValue::Nothing => "nothing".to_string(),
        }
    }

    pub fn as_isize(&self, vm: &Vm) -> Result<isize, VMError> {
        match self {
            RawValue::I32(x) => Ok(x.value as isize),
            RawValue::I64(x) => Ok(x.value as isize),
            any => {
                return Err(error::throw(
                    VMErrorType::TypeMismatch {
                        expected: "u32 or u64".to_string(),
                        received: any.get_type_string(),
                    },
                    vm,
                ));
            }
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            RawValue::I32(_) => None,
            RawValue::I64(_) => None,
            RawValue::U32(x) => Some(x.value as usize),
            RawValue::U64(x) => Some(x.value as usize),
            RawValue::F64(_) => None,
            RawValue::Utf8(_) => None,
            RawValue::Bool(_) => None,
            RawValue::Byte(_) => None,
            RawValue::Nothing => None,
        }
    }

    pub fn as_byte(&self, vm: &Vm) -> Result<Byte, VMError> {
        match self {
            RawValue::Byte(b) => Ok(b.clone()),
            _ => Err(error::throw(
                VMErrorType::TypeMismatch {
                    expected: "bool".to_string(),
                    received: self.get_type_string(),
                },
                vm,
            )),
        }
    }
}
