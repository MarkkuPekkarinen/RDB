use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::query::ColumnDef;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableInfo {
    pub name: String,
    pub root_page_id: u32,
    pub index_root_page_id: u32,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Catalog {
    pub tables: HashMap<String, TableInfo>,
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn add_table(&mut self, table: TableInfo) {
        self.tables.insert(table.name.clone(), table);
    }

    pub fn get_table(&self, name: &str) -> Option<&TableInfo> {
        self.tables.get(name)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Find null terminator or end of valid JSON?
        // For now assume the whole page is valid JSON or padded with 0s which JSON parser might dislike.
        // We should store length prefix.
        if bytes.is_empty() {
            return Ok(Catalog::new());
        }
        
        // Trim trailing nulls
        let len = bytes.iter().position(|&x| x == 0).unwrap_or(bytes.len());
        if len == 0 {
             return Ok(Catalog::new());
        }
        
        let catalog: Catalog = serde_json::from_slice(&bytes[0..len])?;
        Ok(catalog)
    }
}
