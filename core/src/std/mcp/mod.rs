pub mod members;
pub mod types;
use crate::{memory::MemObject, std::mcp::members::init_obj};

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("init".to_string(), init_obj()));

    ("mcp".to_string(), fields)
}
