use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;
use crate::storage::page::{Page, PAGE_SIZE};
use crate::storage::header::DatabaseHeader;
use anyhow::{Result, anyhow};

use std::sync::atomic::{AtomicU32, Ordering};

pub struct Pager {
    file: Mutex<File>,
    pub total_pages: AtomicU32,
}

impl Pager {
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        let file_len = file.metadata()?.len();
        let total_pages = (file_len / PAGE_SIZE as u64) as u32;

        Ok(Self {
            file: Mutex::new(file),
            total_pages: AtomicU32::new(total_pages),
        })
    }

    pub fn read_page(&self, page_id: u32) -> Result<Page> {
        let mut file = self.file.lock().map_err(|_| anyhow!("Lock poisoned"))?;
        let total_pages = self.total_pages.load(Ordering::SeqCst);
        
        if page_id >= total_pages {
             return Err(anyhow!("Page ID {} out of bounds (total: {})", page_id, total_pages));
        }

        file.seek(SeekFrom::Start((page_id as u64) * (PAGE_SIZE as u64)))?;
        
        let mut buffer = [0u8; PAGE_SIZE];
        file.read_exact(&mut buffer)?;

        Ok(Page::from_bytes(page_id, buffer))
    }

    pub fn write_page(&self, page: &Page) -> Result<()> {
        let mut file = self.file.lock().map_err(|_| anyhow!("Lock poisoned"))?;
        
        file.seek(SeekFrom::Start((page.id as u64) * (PAGE_SIZE as u64)))?;
        file.write_all(&page.data)?;
        
        Ok(())
    }
    
    pub fn allocate_page(&self) -> Result<u32> {
        let mut file = self.file.lock().map_err(|_| anyhow!("Lock poisoned"))?;
        let page_id = self.total_pages.fetch_add(1, Ordering::SeqCst);
        
        // Write empty page to extend file
        let page = Page::new(page_id);
        file.seek(SeekFrom::Start((page_id as u64) * (PAGE_SIZE as u64)))?;
        file.write_all(&page.data)?;
        
        Ok(page_id)
    }

    #[allow(dead_code)]
    pub fn read_header(&self) -> Result<DatabaseHeader> {
        let page0 = self.read_page(0)?;
        DatabaseHeader::from_bytes(&page0.data)
    }

    pub fn write_header(&self, header: &DatabaseHeader) -> Result<()> {
        let bytes = header.to_bytes()?;
        let page = Page::from_bytes(0, bytes);
        self.write_page(&page)
    }
}
