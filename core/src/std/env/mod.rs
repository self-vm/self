mod members;
use crate::{
    memory::MemObject,
    std::env::members::{get_obj, read_obj, set_obj},
};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("set".to_string(), set_obj()));
    fields.push(("get".to_string(), get_obj()));
    fields.push(("read".to_string(), read_obj()));

    ("env".to_string(), fields)
}
