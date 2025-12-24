use crate::{
    memory::MemObject,
    std::{
        web::members::{browser_def, browser_obj},
        NativeModuleDef,
    },
};
pub mod members;
pub mod types;

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![browser_def()];

    NativeModuleDef {
        module: "web".to_string(),
        members,
    }
}

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("browser".to_string(), browser_obj()));

    ("web".to_string(), fields)
}
