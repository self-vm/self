use crate::{memory::MemObject, std::web::members::open_obj};
pub mod members;

pub fn generate_struct() -> (String, Vec<(String, MemObject)>) {
    let mut fields = vec![];

    fields.push(("open".to_string(), open_obj()));
    //fields.push(("navigate".to_string(), get_obj()));
    //fields.push(("read".to_string(), get_obj()));

    ("web".to_string(), fields)
}
