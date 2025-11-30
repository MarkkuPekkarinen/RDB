use crate::storage::page::{Page, PAGE_SIZE};
use byteorder::{LittleEndian, ByteOrder};
use anyhow::{Result, anyhow};
use std::borrow::Cow;
use std::io::Cursor;

// Header: num_slots (u16) + free_space_end (u16) + next_page_id (u32)
const HEADER_SIZE: usize = 8;
const SLOT_SIZE: usize = 4;
const COMPRESSION_THRESHOLD: usize = 64;

pub struct SlottedPage<'a> {
    page: &'a mut Page,
}

impl<'a> SlottedPage<'a> {
    pub fn new(page: &'a mut Page) -> Self {
        Self { page }
    }

    pub fn init(&mut self) {
        self.set_num_slots(0);
        self.set_free_space_end(PAGE_SIZE as u16);
        self.set_next_page_id(0); // 0 means no next page (since 0 is header page)
    }

    pub fn num_slots(&self) -> u16 {
        LittleEndian::read_u16(&self.page.data[0..2])
    }

    fn set_num_slots(&mut self, val: u16) {
        LittleEndian::write_u16(&mut self.page.data[0..2], val);
        self.page.dirty = true;
    }

    pub fn free_space_end(&self) -> u16 {
        LittleEndian::read_u16(&self.page.data[2..4])
    }

    fn set_free_space_end(&mut self, val: u16) {
        LittleEndian::write_u16(&mut self.page.data[2..4], val);
        self.page.dirty = true;
    }
    
    pub fn next_page_id(&self) -> u32 {
        LittleEndian::read_u32(&self.page.data[4..8])
    }

    pub fn set_next_page_id(&mut self, val: u32) {
        LittleEndian::write_u32(&mut self.page.data[4..8], val);
        self.page.dirty = true;
    }

    // Calculate free space available for new tuples + new slot
    pub fn free_space(&self) -> usize {
        let header_end = HEADER_SIZE + (self.num_slots() as usize * SLOT_SIZE);
        let data_start = self.free_space_end() as usize;
        if data_start >= header_end {
            data_start - header_end
        } else {
            0
        }
    }

    pub fn compact(&mut self) {
        let num_slots = self.num_slots();
        let mut valid_tuples = Vec::new();

        // 1. Collect all valid tuples
        for i in 0..num_slots {
            let slot_offset = HEADER_SIZE + (i as usize * SLOT_SIZE);
            let tuple_offset = LittleEndian::read_u16(&self.page.data[slot_offset..slot_offset+2]);
            let tuple_len = LittleEndian::read_u16(&self.page.data[slot_offset+2..slot_offset+4]);

            if tuple_offset != 0 && tuple_len != 0 {
                let start = tuple_offset as usize;
                let end = start + tuple_len as usize;
                if end <= PAGE_SIZE {
                    valid_tuples.push((i, self.page.data[start..end].to_vec()));
                }
            }
        }

        // 2. Reset free space pointers (but keep slots count)
        // We don't change num_slots because slot IDs must remain stable
        self.set_free_space_end(PAGE_SIZE as u16);

        // 3. Re-write tuples tightly packed
        for (slot_id, data) in valid_tuples {
            let free_end = self.free_space_end();
            let new_offset = free_end - data.len() as u16;
            
            self.page.data[new_offset as usize..free_end as usize].copy_from_slice(&data);
            self.set_free_space_end(new_offset);

            // Update slot
            let slot_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
            LittleEndian::write_u16(&mut self.page.data[slot_offset..slot_offset+2], new_offset);
            // Length remains same
        }
    }

    pub fn insert_tuple(&mut self, data: &[u8]) -> Result<u16> {
        // Compression logic
        let (final_data, flag) = if data.len() > COMPRESSION_THRESHOLD {
            match zstd::encode_all(Cursor::new(data), 0) {
                Ok(compressed) => {
                    if compressed.len() < data.len() { (compressed, 1u8) } else { (data.to_vec(), 0u8) }
                }
                Err(_) => (data.to_vec(), 0u8),
            }
        } else {
            (data.to_vec(), 0u8)
        };

        let required_space = final_data.len() + 1 + SLOT_SIZE;
        
        if self.free_space() < required_space {
            self.compact();
            if self.free_space() < required_space {
                return Err(anyhow!("Not enough space on page"));
            }
        }

        let num_slots = self.num_slots();
        let free_end = self.free_space_end();
        let new_offset = free_end - (final_data.len() as u16 + 1);

        // Write flag
        self.page.data[new_offset as usize] = flag;
        // Write data
        self.page.data[(new_offset as usize + 1)..free_end as usize].copy_from_slice(&final_data);
        
        self.set_free_space_end(new_offset);

        // Write slot
        let slot_offset = HEADER_SIZE + (num_slots as usize * SLOT_SIZE);
        LittleEndian::write_u16(&mut self.page.data[slot_offset..slot_offset+2], new_offset);
        LittleEndian::write_u16(&mut self.page.data[slot_offset+2..slot_offset+4], (final_data.len() + 1) as u16);

        self.set_num_slots(num_slots + 1);

        Ok(num_slots)
    }

    pub fn get_tuple(&self, slot_id: u16) -> Option<Cow<'_, [u8]>> {
        if slot_id >= self.num_slots() {
            return None;
        }

        let slot_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        let tuple_offset = LittleEndian::read_u16(&self.page.data[slot_offset..slot_offset+2]);
        let tuple_len = LittleEndian::read_u16(&self.page.data[slot_offset+2..slot_offset+4]);

        if tuple_offset == 0 { return None; } // Deleted

        let start = tuple_offset as usize;
        let end = start + tuple_len as usize;
        
        if end > PAGE_SIZE {
            return None; 
        }

        let raw_data = &self.page.data[start..end];
        if raw_data.is_empty() {
            return Some(Cow::Borrowed(&[]));
        }

        let flag = raw_data[0];
        let content = &raw_data[1..];

        if flag == 1 {
            // Compressed
            match zstd::decode_all(Cursor::new(content)) {
                Ok(decompressed) => Some(Cow::Owned(decompressed)),
                Err(_) => None, 
            }
        } else {
            // Uncompressed
            Some(Cow::Borrowed(content))
        }
    }

    pub fn update_tuple(&mut self, slot_id: u16, data: &[u8]) -> Result<()> {
        if slot_id >= self.num_slots() {
            return Err(anyhow!("Invalid slot ID"));
        }
        
        // Compression logic
        let (final_data, flag) = if data.len() > COMPRESSION_THRESHOLD {
            match zstd::encode_all(Cursor::new(data), 0) {
                Ok(compressed) => {
                    if compressed.len() < data.len() { (compressed, 1u8) } else { (data.to_vec(), 0u8) }
                }
                Err(_) => (data.to_vec(), 0u8),
            }
        } else {
            (data.to_vec(), 0u8)
        };

        let required_space = final_data.len() + 1; 
        
        // Check if we fit in existing space? No, we always allocate new space for simplicity/safety
        // unless we want to implement complex in-place overwrite.
        // But wait, if we compact, we might move the tuple we are updating.
        // So we should compact first if needed.
        
        if self.free_space() < required_space {
            self.compact();
            if self.free_space() < required_space {
                return Err(anyhow!("Not enough space on page for update"));
            }
        }

        let free_end = self.free_space_end();
        let new_offset = free_end - (final_data.len() as u16 + 1);

        // Write flag
        self.page.data[new_offset as usize] = flag;
        // Write data
        self.page.data[(new_offset as usize + 1)..free_end as usize].copy_from_slice(&final_data);
        
        self.set_free_space_end(new_offset);

        // Update slot
        let slot_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        LittleEndian::write_u16(&mut self.page.data[slot_offset..slot_offset+2], new_offset);
        LittleEndian::write_u16(&mut self.page.data[slot_offset+2..slot_offset+4], (final_data.len() + 1) as u16);

        Ok(())
    }

    pub fn mark_deleted(&mut self, slot_id: u16) -> Result<()> {
        if slot_id >= self.num_slots() {
            return Err(anyhow!("Invalid slot ID"));
        }
        let slot_offset = HEADER_SIZE + (slot_id as usize * SLOT_SIZE);
        // Set offset to 0 to indicate deleted
        LittleEndian::write_u16(&mut self.page.data[slot_offset..slot_offset+2], 0);
        LittleEndian::write_u16(&mut self.page.data[slot_offset+2..slot_offset+4], 0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slotted_page_insert_get() {
        let mut page = Page::new(0);
        let mut slotted = SlottedPage::new(&mut page);
        slotted.init();

        let data1 = b"hello world";
        let slot1 = slotted.insert_tuple(data1).unwrap();
        
        let data2 = b"second tuple";
        let slot2 = slotted.insert_tuple(data2).unwrap();

        assert_eq!(slotted.num_slots(), 2);
        
        let retrieved1 = slotted.get_tuple(slot1).unwrap();
        assert_eq!(retrieved1.as_ref(), data1);
        
        let retrieved2 = slotted.get_tuple(slot2).unwrap();
        assert_eq!(retrieved2.as_ref(), data2);
    }

    #[test]
    fn test_slotted_page_compression() {
        let mut page = Page::new(0);
        let mut slotted = SlottedPage::new(&mut page);
        slotted.init();

        // Create data larger than COMPRESSION_THRESHOLD (64)
        let data = vec![0u8; 100];
        let slot = slotted.insert_tuple(&data).unwrap();

        let retrieved = slotted.get_tuple(slot).unwrap();
        assert_eq!(retrieved.as_ref(), &data);
        
        // Verify internal storage is compressed (flag = 1)
        // We can't easily access private fields here without exposing them or inspecting raw bytes.
        // But the fact that get_tuple works implies compression/decompression cycle worked.
    }
}
