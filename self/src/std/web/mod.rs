use crate::{
    memory::MemObject,
    std::{
        web::members::{open_def, open_obj},
        NativeModuleDef,
    },
};
pub mod members;

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![open_def()];

    NativeModuleDef {
        module: "web".to_string(),
        members,
    }
}

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("open".to_string(), open_obj()));
    //fields.push(("navigate".to_string(), get_obj()));
    //fields.push(("read".to_string(), get_obj()));

    ("web".to_string(), fields)
}
