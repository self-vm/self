use crate::{
    memory::MemObject,
    std::{native::members::load_lib_obj, NativeModuleDef},
};

mod members;
pub mod types;

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("load_lib".to_string(), load_lib_obj()));

    ("native".to_string(), fields)
}

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![];

    NativeModuleDef {
        module: "native".to_string(),
        members,
    }
}
