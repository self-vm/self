#[derive(Debug, Clone)]
pub struct SelfString {
    pub value: String,
}

impl SelfString {
    pub fn new(value: String) -> SelfString {
        SelfString { value }
    }
    pub fn to_string(&self) -> String {
        self.value.clone()
    }
}
