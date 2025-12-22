mod members;
pub mod types;
use crate::{memory::MemObject, std::schedule::members::interval_obj};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("interval".to_string(), interval_obj()));

    ("schedule".to_string(), fields)
}
