#[derive(Debug)]
pub enum TypeError {
    InvalidArgsCount { expected: u32, received: u32 },
    InvalidTypeUnwrap { expected: String, received: String },
}
