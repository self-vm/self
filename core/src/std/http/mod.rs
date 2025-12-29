/*
THIS MODULE IN THE FUTURE PROBABLY WILL BE
CODED IN EGO WITH THE NET LIB, BUT FOR THE
MOMENT TO MOVE FAST LET'S BUILD IT IN RUST
*/

use crate::{
    memory::MemObject,
    std::{
        http::members::{get_def, get_obj},
        NativeModuleDef,
    },
};

mod members;

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("get".to_string(), get_obj()));

    ("http".to_string(), fields)
}

pub fn generate_mod_def() -> NativeModuleDef {
    let members = vec![get_def()];

    NativeModuleDef {
        module: "http".to_string(),
        members,
    }
}
