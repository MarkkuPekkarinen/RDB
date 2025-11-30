use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::storage::page::PAGE_SIZE;
use anyhow::{Result, anyhow};

pub const MAGIC: &[u8; 7] = b"RDBFILE";
pub const CURRENT_FILE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHeader {
    pub magic: [u8; 7],
    pub file_format_version: u32,
    pub rdb_engine_version: String,
    pub page_size: u32,
    pub created_at: i64,
    pub last_opened_at: i64,
    pub last_opened_with_engine: String,
    pub database_name: String,
    pub wal_enabled: bool,
    pub encryption: bool,
    pub root_catalog_page: u32,
}

impl DatabaseHeader {
    pub fn new(name: String) -> Self {
        Self {
            magic: *MAGIC,
            file_format_version: CURRENT_FILE_FORMAT_VERSION,
            rdb_engine_version: env!("CARGO_PKG_VERSION").to_string(),
            page_size: PAGE_SIZE as u32,
            created_at: chrono::Utc::now().timestamp(),
            last_opened_at: chrono::Utc::now().timestamp(),
            last_opened_with_engine: env!("CARGO_PKG_VERSION").to_string(),
            database_name: name,
            wal_enabled: true,
            encryption: false,
            root_catalog_page: 1, // Page 1 is usually the start of the catalog
        }
    }

    pub fn to_bytes(&self) -> Result<[u8; PAGE_SIZE]> {
        let mut bytes = [0u8; PAGE_SIZE];
        let mut cursor = Cursor::new(&mut bytes[..]);

        cursor.write_all(&self.magic)?;
        cursor.write_u32::<LittleEndian>(self.file_format_version)?;
        
        let engine_ver_bytes = self.rdb_engine_version.as_bytes();
        cursor.write_u16::<LittleEndian>(engine_ver_bytes.len() as u16)?;
        cursor.write_all(engine_ver_bytes)?;

        cursor.write_u32::<LittleEndian>(self.page_size)?;
        cursor.write_i64::<LittleEndian>(self.created_at)?;
        cursor.write_i64::<LittleEndian>(self.last_opened_at)?;

        let last_engine_ver_bytes = self.last_opened_with_engine.as_bytes();
        cursor.write_u16::<LittleEndian>(last_engine_ver_bytes.len() as u16)?;
        cursor.write_all(last_engine_ver_bytes)?;

        let name_bytes = self.database_name.as_bytes();
        cursor.write_u16::<LittleEndian>(name_bytes.len() as u16)?;
        cursor.write_all(name_bytes)?;

        cursor.write_u8(if self.wal_enabled { 1 } else { 0 })?;
        cursor.write_u8(if self.encryption { 1 } else { 0 })?;
        cursor.write_u32::<LittleEndian>(self.root_catalog_page)?;

        Ok(bytes)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8; PAGE_SIZE]) -> Result<Self> {
        let mut cursor = Cursor::new(&bytes[..]);
        
        let mut magic = [0u8; 7];
        cursor.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(anyhow!("Invalid magic bytes"));
        }

        let file_format_version = cursor.read_u32::<LittleEndian>()?;
        
        let engine_ver_len = cursor.read_u16::<LittleEndian>()?;
        let mut engine_ver_bytes = vec![0u8; engine_ver_len as usize];
        cursor.read_exact(&mut engine_ver_bytes)?;
        let rdb_engine_version = String::from_utf8(engine_ver_bytes)?;

        let page_size = cursor.read_u32::<LittleEndian>()?;
        if page_size as usize != PAGE_SIZE {
             return Err(anyhow!("Page size mismatch"));
        }

        let created_at = cursor.read_i64::<LittleEndian>()?;
        let last_opened_at = cursor.read_i64::<LittleEndian>()?;

        let last_engine_ver_len = cursor.read_u16::<LittleEndian>()?;
        let mut last_engine_ver_bytes = vec![0u8; last_engine_ver_len as usize];
        cursor.read_exact(&mut last_engine_ver_bytes)?;
        let last_opened_with_engine = String::from_utf8(last_engine_ver_bytes)?;

        let name_len = cursor.read_u16::<LittleEndian>()?;
        let mut name_bytes = vec![0u8; name_len as usize];
        cursor.read_exact(&mut name_bytes)?;
        let database_name = String::from_utf8(name_bytes)?;

        let wal_enabled = cursor.read_u8()? != 0;
        let encryption = cursor.read_u8()? != 0;
        let root_catalog_page = cursor.read_u32::<LittleEndian>()?;

        Ok(Self {
            magic,
            file_format_version,
            rdb_engine_version,
            page_size,
            created_at,
            last_opened_at,
            last_opened_with_engine,
            database_name,
            wal_enabled,
            encryption,
            root_catalog_page,
        })
    }
}
