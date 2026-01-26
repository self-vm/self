mod members;

use crate::{
    memory::MemObject,
    std::selfmod::members::{get_stack_fn_ref, modules_string_obj, sleep_ref},
};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("get_stack".to_string(), get_stack_fn_ref()));
    fields.push(("modules_string".to_string(), modules_string_obj()));
    fields.push(("sleep".to_string(), sleep_ref()));

    ("self".to_string(), fields)
}
