#[derive(Debug)]
pub enum VectorError {
    IndexOutOfBounds {
        index: usize,
        length: usize,
    },
}
