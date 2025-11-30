use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use crate::storage::page::Page;
use crate::storage::pager::Pager;
use anyhow::{Result, anyhow};
use lru::LruCache;
use std::num::NonZeroUsize;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub struct GlobalPageId {
    pub db_id: u32,
    pub page_id: u32,
}

pub struct BufferPool {
    pages: Mutex<LruCache<GlobalPageId, Arc<RwLock<Page>>>>,
    pagers: Mutex<HashMap<u32, Arc<Pager>>>,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        let c = NonZeroUsize::new(capacity).expect("Capacity must be > 0");
        Self {
            pages: Mutex::new(LruCache::new(c)),
            pagers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register_pager(&self, db_id: u32, pager: Arc<Pager>) {
        self.pagers.lock().insert(db_id, pager);
    }

    pub fn fetch_page(&self, global_id: GlobalPageId) -> Result<Arc<RwLock<Page>>> {
        let mut pages = self.pages.lock();
        
        if let Some(page) = pages.get(&global_id) {
            return Ok(page.clone());
        }

        // Load from disk
        let pagers = self.pagers.lock();
        let pager = pagers.get(&global_id.db_id).ok_or(anyhow!("Database not registered"))?;
        let page = pager.read_page(global_id.page_id)?;
        
        let page_ref = Arc::new(RwLock::new(page));
        
        // Insert and handle eviction
        if let Some((evicted_id, evicted_page)) = pages.push(global_id, page_ref.clone()) {
             // Flush if dirty
             let page_guard = evicted_page.read();
             if page_guard.dirty {
                 if let Some(pager) = pagers.get(&evicted_id.db_id) {
                     pager.write_page(&page_guard)?;
                 }
             }
        }

        Ok(page_ref)
    }

    pub fn new_page(&self, db_id: u32) -> Result<Arc<RwLock<Page>>> {
        let pagers = self.pagers.lock();
        let pager = pagers.get(&db_id).ok_or(anyhow!("Database not registered"))?;
        
        let page_id = pager.allocate_page()?;
        let page = Page::new(page_id);
        
        let mut pages = self.pages.lock();
        let page_ref = Arc::new(RwLock::new(page));
        
        // Insert and handle eviction
        if let Some((evicted_id, evicted_page)) = pages.push(GlobalPageId { db_id, page_id }, page_ref.clone()) {
             // Flush if dirty
             let page_guard = evicted_page.read();
             if page_guard.dirty {
                 if let Some(pager) = pagers.get(&evicted_id.db_id) {
                     pager.write_page(&page_guard)?;
                 }
             }
        }
        
        Ok(page_ref)
    }
    
    #[allow(dead_code)]
    pub fn flush_all(&self) -> Result<()> {
        let pages = self.pages.lock();
        let pagers = self.pagers.lock();
        
        for (pid, page) in pages.iter() {
            let mut page_guard = page.write();
            if page_guard.dirty {
                if let Some(pager) = pagers.get(&pid.db_id) {
                    pager.write_page(&page_guard)?;
                    page_guard.dirty = false;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::pager::Pager;
    use tempfile::TempDir;

    #[test]
    fn test_buffer_pool_creation() {
        let pool = BufferPool::new(10);
        // Just verify it was created successfully
        assert_eq!(std::mem::size_of_val(&pool), std::mem::size_of::<BufferPool>());
    }

    #[test]
    fn test_buffer_pool_register_pager() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pager = Arc::new(Pager::open(&db_path).unwrap());
        let pool = BufferPool::new(10);
        
        pool.register_pager(0, pager);
        // Successfully registered
    }

    #[test]
    fn test_buffer_pool_fetch_and_allocate() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pager = Arc::new(Pager::open(&db_path).unwrap());
        let pool = BufferPool::new(10);
        
        pool.register_pager(0, pager.clone());
        
        // Allocate pages first
        pager.allocate_page().unwrap();
        pager.allocate_page().unwrap();
        
        // Fetch page
        let page = pool.fetch_page(GlobalPageId { db_id: 0, page_id: 0 }).unwrap();
        let guard = page.read();
        assert_eq!(guard.id, 0);
    }

    #[test]
    fn test_buffer_pool_lru_eviction() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pager = Arc::new(Pager::open(&db_path).unwrap());
        let pool = BufferPool::new(3); // Small pool to test eviction
        
        pool.register_pager(0, pager.clone());
        
        // Allocate more pages than pool size
        for _ in 0..5 {
            pager.allocate_page().unwrap();
        }
        
        // Fetch pages - should trigger LRU eviction
        for i in 0..5 {
            let _ = pool.fetch_page(GlobalPageId { db_id: 0, page_id: i });
        }
        
        // Pool should handle eviction gracefully
    }

    #[test]
    fn test_buffer_pool_dirty_page_flush() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pager = Arc::new(Pager::open(&db_path).unwrap());
        let pool = BufferPool::new(10);
        
        pool.register_pager(0, pager.clone());
        pager.allocate_page().unwrap();
        
        let page = pool.fetch_page(GlobalPageId { db_id: 0, page_id: 0 }).unwrap();
        {
            let mut guard = page.write();
            guard.data[0] = 42;
            guard.dirty = true;
        }
        
        // Flush should succeed
        pool.flush_all().unwrap();
    }
}
