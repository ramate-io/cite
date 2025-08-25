#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id {
    data: [u8; 64],
    len: usize,
}

impl Id {
    pub fn new(id: &str) -> Self {
        let bytes = id.as_bytes();
        let mut data = [0u8; 64];
        let len = bytes.len().min(64);
        data[..len].copy_from_slice(&bytes[..len]);
        Self { data, len }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len]
    }
}