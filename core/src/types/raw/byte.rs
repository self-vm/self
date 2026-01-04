#[derive(Debug, Clone)]
pub struct Byte {
    pub value: u8,
}
impl Byte {
    pub fn new(value: u8) -> Byte {
        Byte { value }
    }
}
