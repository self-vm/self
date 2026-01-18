#[derive(Debug)]
pub enum FatalError {
    InvalidPropertyAccess { object: String, property: String },
    InvalidValueUnwrap(String),
}
