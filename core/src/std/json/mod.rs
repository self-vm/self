use crate::memory::MemObject;
pub mod members;

use members::encode_obj;

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("encode".to_string(), encode_obj()));
    // fields.push(("decode".to_string(), decode_obj()));

    ("json".to_string(), fields)
}

