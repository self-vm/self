#[derive(Debug)]
pub enum ByteError {
    OutOfBounds { received: isize },
}
