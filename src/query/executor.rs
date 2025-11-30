use std::sync::Arc;
use crate::query::{Query, CreateTableQuery, InsertQuery, SelectQuery, UpdateQuery, DeleteQuery, DropTableQuery};
use crate::storage::buffer::{BufferPool, GlobalPageId};
use crate::storage::catalog::{Catalog, TableInfo};
use crate::storage::slotted::SlottedPage;
use crate::storage::index::BTreeIndex;
use anyhow::{Result, anyhow};
use serde_json::Value;

pub enum ExecutionResult {
    Message(String),
    Json(Value),
}

pub struct Executor {
    buffer_pool: Arc<BufferPool>,
}

impl Executor {
    pub fn new(buffer_pool: Arc<BufferPool>) -> Self {
        Self { buffer_pool }
    }

    pub fn execute(&self, query: Query) -> Result<ExecutionResult> {
        match query {
            Query::CreateTable(q) => self.handle_create_table(q),
            Query::DropTable(q) => self.handle_drop_table(q),
            Query::Insert(q) => self.handle_insert(q),
            Query::Select(q) => self.handle_select(q),
            Query::Update(q) => self.handle_update(q),
            Query::Delete(q) => self.handle_delete(q),
            Query::Batch(queries) => self.handle_batch(queries),
        }
    }

    fn handle_batch(&self, queries: Vec<Query>) -> Result<ExecutionResult> {
        let mut results = Vec::new();
        for query in queries {
            match self.execute(query)? {
                ExecutionResult::Message(msg) => results.push(Value::String(msg)),
                ExecutionResult::Json(val) => results.push(val),
            }
        }
        Ok(ExecutionResult::Json(Value::Array(results)))
    }

    // Helper for filtering
    fn check_filter(val: &Value, where_clause: &crate::query::WhereClause) -> bool {
        if let Some(col_val) = val.get(&where_clause.column) {
            match where_clause.cmp.as_str() {
                "=" => col_val == &where_clause.value,
                "!=" => col_val != &where_clause.value,
                ">" => {
                    if let (Some(a), Some(b)) = (col_val.as_f64(), where_clause.value.as_f64()) {
                        a > b
                    } else { false }
                },
                "<" => {
                    if let (Some(a), Some(b)) = (col_val.as_f64(), where_clause.value.as_f64()) {
                        a < b
                    } else { false }
                },
                ">=" => {
                    if let (Some(a), Some(b)) = (col_val.as_f64(), where_clause.value.as_f64()) {
                        a >= b
                    } else { false }
                },
                "<=" => {
                    if let (Some(a), Some(b)) = (col_val.as_f64(), where_clause.value.as_f64()) {
                        a <= b
                    } else { false }
                },
                "LIKE" => {
                    if let (Some(s), Some(pattern)) = (col_val.as_str(), where_clause.value.as_str()) {
                        // Simple wildcard support: % at start/end
                        if pattern.starts_with('%') && pattern.ends_with('%') {
                            s.contains(&pattern[1..pattern.len()-1])
                        } else if pattern.starts_with('%') {
                            s.ends_with(&pattern[1..])
                        } else if pattern.ends_with('%') {
                            s.starts_with(&pattern[..pattern.len()-1])
                        } else {
                            s == pattern
                        }
                    } else { false }
                },
                "IN" => {
                    if let Some(arr) = where_clause.value.as_array() {
                        arr.contains(col_val)
                    } else { false }
                },
                _ => false,
            }
        } else {
            false
        }
    }

    fn get_db_id(&self, _db_name: &str) -> Result<u32> {
        if _db_name == "main" {
            Ok(0)
        } else {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            _db_name.hash(&mut hasher);
            Ok(hasher.finish() as u32)
        }
    }

    fn handle_create_table(&self, query: CreateTableQuery) -> Result<ExecutionResult> {
        let db_id = self.get_db_id(&query.database)?;
        
        // 1. Load Catalog (Page 1)
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let mut catalog_guard = catalog_page.write();
        
        let mut catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        if catalog.tables.contains_key(&query.table) {
            return Err(anyhow!("Table {} already exists", query.table));
        }

        // 2. Allocate root page for table
        let root_page = self.buffer_pool.new_page(db_id)?;
        let root_page_id = root_page.read().id;
        {
            let mut root_guard = root_page.write();
            let mut slotted = SlottedPage::new(&mut root_guard);
            slotted.init();
        }
        
        // 3. Allocate Index Root Page
        let index_root_page = self.buffer_pool.new_page(db_id)?;
        let index_root_page_id = index_root_page.read().id;
        
        // Init Index
        let index = BTreeIndex::new(self.buffer_pool.clone(), db_id, index_root_page_id);
        index.init()?;

        // 4. Update Catalog
        let table_info = TableInfo {
            name: query.table.clone(),
            root_page_id,
            index_root_page_id,
            columns: query.columns,
        };
        catalog.add_table(table_info);
        
        let bytes = catalog.to_bytes()?;
        if bytes.len() > crate::storage::page::PAGE_SIZE {
            return Err(anyhow!("Catalog too large for single page"));
        }
        
        catalog_guard.data[..bytes.len()].copy_from_slice(&bytes);
        if bytes.len() < crate::storage::page::PAGE_SIZE {
            catalog_guard.data[bytes.len()..].fill(0);
        }
        catalog_guard.dirty = true;

        Ok(ExecutionResult::Message(format!("Table {} created", query.table)))
    }

    fn handle_drop_table(&self, query: DropTableQuery) -> Result<ExecutionResult> {
        let db_id = self.get_db_id(&query.database)?;
        
        // 1. Load Catalog
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let mut catalog_guard = catalog_page.write();
        let mut catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        if !catalog.tables.contains_key(&query.table) {
            return Err(anyhow!("Table {} not found", query.table));
        }
        
        // 2. Remove from Catalog
        // Note: In a real DB, we would also free the pages (root_page_id, index_root_page_id, and all data pages)
        // For this MVP, we just remove the metadata entry. The pages leak but are inaccessible.
        catalog.tables.remove(&query.table);
        
        let bytes = catalog.to_bytes()?;
        catalog_guard.data[..bytes.len()].copy_from_slice(&bytes);
        if bytes.len() < crate::storage::page::PAGE_SIZE {
            catalog_guard.data[bytes.len()..].fill(0);
        }
        catalog_guard.dirty = true;
        
        Ok(ExecutionResult::Message(format!("Table {} dropped", query.table)))
    }

    fn handle_insert(&self, query: InsertQuery) -> Result<ExecutionResult> {
        let db_id = self.get_db_id(&query.database)?;
        
        // 1. Load Catalog
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let catalog_guard = catalog_page.read();
        let catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        let table_info = catalog.get_table(&query.table)
            .ok_or(anyhow!("Table {} not found", query.table))?;
            
        // Find PK column
        let pk_col = table_info.columns.iter().find(|c| c.primary_key);
            
        // 2. Insert values
        let mut current_page_id = table_info.root_page_id;
        
        for value in query.values {
            let tuple_data = serde_json::to_vec(&value)?;
            
            #[allow(unused_assignments)]
            let mut inserted_page_id = None;
            #[allow(unused_assignments)]
            let mut inserted_slot_id = None;
            
            loop {
                let page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: current_page_id })?;
                let mut page_guard = page.write();
                let mut slotted = SlottedPage::new(&mut page_guard);
                
                match slotted.insert_tuple(&tuple_data) {
                    Ok(slot_id) => {
                        inserted_page_id = Some(current_page_id);
                        inserted_slot_id = Some(slot_id);
                        break;
                    },
                    Err(_) => {
                        // Page full
                        let next = slotted.next_page_id();
                        if next != 0 {
                            current_page_id = next;
                            continue;
                        } else {
                            // Allocate new page
                            drop(slotted); // Release borrow
                            
                            let new_page = self.buffer_pool.new_page(db_id)?;
                            let new_page_id = new_page.read().id;
                            
                            {
                                let mut new_guard = new_page.write();
                                let mut new_slotted = SlottedPage::new(&mut new_guard);
                                new_slotted.init();
                            }
                            
                            // Link
                            let mut slotted = SlottedPage::new(&mut page_guard);
                            slotted.set_next_page_id(new_page_id);
                            
                            current_page_id = new_page_id;
                        }
                    }
                }
            }
            
            // Insert into Index
            if let Some(pk) = pk_col {
                if let Some(val) = value.get(&pk.name) {
                    if let Some(int_val) = val.as_u64() {
                        if let (Some(pid), Some(sid)) = (inserted_page_id, inserted_slot_id) {
                             let key = int_val as u32;
                             let index = BTreeIndex::new(self.buffer_pool.clone(), db_id, table_info.index_root_page_id);
                             index.insert(key, (pid, sid))?;
                        }
                    }
                }
            }
        }

        Ok(ExecutionResult::Message("Inserted".to_string()))
    }

    fn handle_select(&self, query: SelectQuery) -> Result<ExecutionResult> {
        if query.join.is_some() {
            return Err(anyhow!("Joins are not yet implemented"));
        }
        let db_id = self.get_db_id(&query.database)?;
        
        // 1. Load Catalog
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let catalog_guard = catalog_page.read();
        let catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        let table_info = catalog.get_table(&query.from)
            .ok_or(anyhow!("Table {} not found", query.from))?;
            
        let mut results = Vec::new();
        
        // Check for Index Scan
        let pk_col = table_info.columns.iter().find(|c| c.primary_key);
        let mut index_scan = false;
        
        if let Some(pk) = pk_col {
            if let Some(where_clause) = &query.r#where {
                if where_clause.column == pk.name && where_clause.cmp == "=" {
                    if let Some(int_val) = where_clause.value.as_u64() {
                        index_scan = true;
                        let key = int_val as u32;
                        let index = BTreeIndex::new(self.buffer_pool.clone(), db_id, table_info.index_root_page_id);
                        if let Some((pid, sid)) = index.search(key)? {
                            let page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: pid })?;
                            let page_guard = page.read();
                            #[allow(mutable_transmutes)]
                            let page_mut_ref: &mut crate::storage::page::Page = unsafe { std::mem::transmute(&*page_guard) };
                            let slotted = SlottedPage::new(page_mut_ref);
                            
                            if let Some(tuple_bytes) = slotted.get_tuple(sid) {
                                if !tuple_bytes.is_empty() {
                                    let val: Value = serde_json::from_slice(&tuple_bytes)?;
                                // Filter (re-check just in case) and Project
                                // ... (Simplified: assume match)
                                
                                // Project
                                if query.columns.len() == 1 && query.columns[0] == "*" {
                                    results.push(val);
                                } else {
                                    let mut projected = serde_json::Map::new();
                                    for col in &query.columns {
                                        if let Some(v) = val.get(col) {
                                            projected.insert(col.clone(), v.clone());
                                        }
                                    }
                                    results.push(Value::Object(projected));
                                }
                                }
                            }
                        }
                    }
                }
            }
        }
        
            
        if !index_scan {
            // Full Table Scan
            let mut current_page_id = table_info.root_page_id;
            
            while current_page_id != 0 {
                let page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: current_page_id })?;
                let page_guard = page.read();
                #[allow(mutable_transmutes)]
                let page_mut_ref: &mut crate::storage::page::Page = unsafe { std::mem::transmute(&*page_guard) };
                let slotted = SlottedPage::new(page_mut_ref);
                
                let num_slots = slotted.num_slots();
                for i in 0..num_slots {
                    if let Some(tuple_bytes) = slotted.get_tuple(i) {
                        if tuple_bytes.is_empty() { continue; }
                        let val: Value = serde_json::from_slice(&tuple_bytes)?;
                        
                        // Filter logic
                        let mut match_filter = true;
                        if let Some(where_clause) = &query.r#where {
                            match_filter = Self::check_filter(&val, where_clause);
                        }
                        
                        if match_filter {
                            // Project
                            if query.columns.len() == 1 && query.columns[0] == "*" {
                                results.push(val);
                            } else {
                                let mut projected = serde_json::Map::new();
                                for col in &query.columns {
                                    if let Some(v) = val.get(col) {
                                        projected.insert(col.clone(), v.clone());
                                    }
                                }
                                results.push(Value::Object(projected));
                            }
                        }
                    }
                }
                current_page_id = slotted.next_page_id();
            }
        }

        // Apply Order By
        if let Some(order_by) = &query.order_by {
            results.sort_by(|a, b| {
                let val_a = a.get(&order_by.column);
                let val_b = b.get(&order_by.column);
                
                let cmp = match (val_a, val_b) {
                    (Some(va), Some(vb)) => {
                        if let (Some(sa), Some(sb)) = (va.as_str(), vb.as_str()) {
                            sa.cmp(sb)
                        } else if let (Some(fa), Some(fb)) = (va.as_f64(), vb.as_f64()) {
                            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                        } else if let (Some(ia), Some(ib)) = (va.as_i64(), vb.as_i64()) {
                            ia.cmp(&ib)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    },
                    (Some(_), None) => std::cmp::Ordering::Less, // Nulls last
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                };
                
                if order_by.direction.to_uppercase() == "DESC" {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        // Apply Offset and Limit
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        
        let final_results: Vec<Value> = results.into_iter().skip(offset).take(limit).collect();
        
        Ok(ExecutionResult::Json(Value::Array(final_results)))
    }

    fn handle_update(&self, query: UpdateQuery) -> Result<ExecutionResult> {
        let db_id = self.get_db_id(&query.database)?;
        
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let catalog_guard = catalog_page.read();
        let catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        let table_info = catalog.get_table(&query.table)
            .ok_or(anyhow!("Table {} not found", query.table))?;
            
        let mut current_page_id = table_info.root_page_id;
        let mut updated_count = 0;
        
        while current_page_id != 0 {
            let page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: current_page_id })?;
            let mut page_guard = page.write();
            let mut slotted = SlottedPage::new(&mut page_guard);
            
            let num_slots = slotted.num_slots();
            for i in 0..num_slots {
                if let Some(tuple_bytes) = slotted.get_tuple(i) {
                    if tuple_bytes.is_empty() { continue; }
                    let mut val: Value = serde_json::from_slice(&tuple_bytes)?;
                    
                    // Filter
                    let mut match_filter = true;
                    if let Some(where_clause) = &query.r#where {
                        match_filter = Self::check_filter(&val, where_clause);
                    }
                    
                    if match_filter {
                        // Update values
                        if let Value::Object(ref mut map) = val {
                            for (k, v) in &query.set {
                                map.insert(k.clone(), v.clone());
                            }
                        }
                        
                        let new_bytes = serde_json::to_vec(&val)?;
                        slotted.update_tuple(i, &new_bytes)?;
                        updated_count += 1;
                    }
                }
            }
            current_page_id = slotted.next_page_id();
        }
        
        Ok(ExecutionResult::Message(format!("Updated {} rows", updated_count)))
    }

    fn handle_delete(&self, query: DeleteQuery) -> Result<ExecutionResult> {
        let db_id = self.get_db_id(&query.database)?;
        
        let catalog_page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: 1 })?;
        let catalog_guard = catalog_page.read();
        let catalog = Catalog::from_bytes(&catalog_guard.data)?;
        
        let table_info = catalog.get_table(&query.table)
            .ok_or(anyhow!("Table {} not found", query.table))?;
            
        let mut current_page_id = table_info.root_page_id;
        let mut deleted_count = 0;
        
        while current_page_id != 0 {
            let page = self.buffer_pool.fetch_page(GlobalPageId { db_id, page_id: current_page_id })?;
            let mut page_guard = page.write();
            let mut slotted = SlottedPage::new(&mut page_guard);
            
            let num_slots = slotted.num_slots();
            for i in 0..num_slots {
                if let Some(tuple_bytes) = slotted.get_tuple(i) {
                    if tuple_bytes.is_empty() { continue; }
                    let val: Value = serde_json::from_slice(&tuple_bytes)?;
                    
                    // Filter
                    let mut match_filter = true;
                    if let Some(where_clause) = &query.r#where {
                        match_filter = Self::check_filter(&val, where_clause);
                    }
                    
                    if match_filter {
                        slotted.mark_deleted(i)?;
                        deleted_count += 1;
                    }
                }
            }
            current_page_id = slotted.next_page_id();
        }
        
        Ok(ExecutionResult::Message(format!("Deleted {} rows", deleted_count)))
    }
}
