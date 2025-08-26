#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Hash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl Hash {
    pub fn from_data(initial: Vec<u8>, current: Vec<u8>) -> Self {
        Self { initial, current }
    }
}
