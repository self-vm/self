mod members;

use crate::{
    memory::MemObject,
    std::{io::members::read_line_obj, NativeModuleDef},
};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("read_line".to_string(), read_line_obj()));

    ("io".to_string(), fields)
}

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![];

    NativeModuleDef {
        module: "io".to_string(),
        members,
    }
}
