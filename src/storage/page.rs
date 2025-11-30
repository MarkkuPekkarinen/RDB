pub const PAGE_SIZE: usize = 8192;

#[derive(Clone)]
pub struct Page {
    pub id: u32,
    pub data: [u8; PAGE_SIZE],
    pub dirty: bool,
    #[allow(dead_code)]
    pub pin_count: u32,
}

impl Page {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            data: [0; PAGE_SIZE],
            dirty: false,
            pin_count: 0,
        }
    }

    pub fn from_bytes(id: u32, bytes: [u8; PAGE_SIZE]) -> Self {
        Self {
            id,
            data: bytes,
            dirty: false,
            pin_count: 0,
        }
    }
}
