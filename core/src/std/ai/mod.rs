mod members;
mod prompts;
mod providers;
pub mod types;

use crate::{
    memory::MemObject,
    opcodes::DataType,
    std::{
        ai::members::{chain_obj, do_fn, infer, infer_def, resolve_def, resolve_obj},
        NativeModuleDef,
    },
    types::object::{
        func::{Engine, Function},
        structs::StructDeclaration,
    },
};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];
    let infer_ref = MemObject::Function(Function::new(
        "infer".to_string(),
        vec![], // TODO: load params to native functions
        Engine::NativeAsync(infer),
    ));
    let do_ref = MemObject::Function(Function::new(
        "do".to_string(),
        vec![], // TODO: load params to native functions
        Engine::NativeAsync(do_fn),
    ));
    let engine_ref = MemObject::StructDeclaration(StructDeclaration {
        identifier: "Engine".to_string(),
        fields: vec![("name".to_string(), DataType::Utf8)],
    });

    fields.push(("infer".to_string(), infer_ref));
    fields.push(("resolve".to_string(), resolve_obj()));
    fields.push(("do".to_string(), do_ref));
    fields.push(("chain".to_string(), chain_obj()));
    fields.push(("Engine".to_string(), engine_ref));

    ("ai".to_string(), fields)
}

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![infer_def(), resolve_def()];

    NativeModuleDef {
        module: "ai".to_string(),
        members,
    }
}
