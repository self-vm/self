pub mod members;
mod prompts;
mod providers;
pub mod types;

use crate::{
    memory::MemObject,
    std::{
        ai::members::{
            chain_obj, default_unfold_obj, do_fn, infer, infer_def, resolve_def, resolve_obj,
        },
        NativeModuleDef,
    },
    types::object::{
        func::{Engine, Function},
        structs::StructDeclaration,
    },
};
use self_bytecode::DataType;

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
    fields.push(("default_unfold".to_string(), default_unfold_obj()));

    ("ai".to_string(), fields)
}

// I don't think it's a good idea to let the Chain
// make another AI call, could end in infinite loops
// or in unnecessary calls.
// pub fn generate_mod_def() -> NativeModuleDef {
//     let members = vec![resolve_def()];

//     NativeModuleDef {
//         module: "ai".to_string(),
//         members,
//     }
// }
