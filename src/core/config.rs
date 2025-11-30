use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    pub indexing: IndexingConfig,
    pub performance: PerformanceConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
    pub limits: LimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub default_db: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub compression_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enable_query_cache: bool,
    pub query_cache_size: usize,
    pub query_cache_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub btree_node_size: usize,
    pub auto_index_primary_keys: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub auto_compact: bool,
    pub compact_threshold: u8,
    pub max_batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub token_expiration: u64,
    pub argon2_memory_cost: u32,
    pub argon2_time_cost: u32,
    pub argon2_parallelism: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub log_to_file: bool,
    pub log_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_result_rows: usize,
    pub max_query_time: u64,
    pub max_payload_size: usize,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 4,
            },
            database: DatabaseConfig {
                default_db: "main".to_string(),
                data_dir: "./data".to_string(),
            },
            storage: StorageConfig {
                page_size: 4096,
                buffer_pool_size: 500,
                compression_threshold: 64,
            },
            cache: CacheConfig {
                enable_query_cache: true,
                query_cache_size: 1000,
                query_cache_ttl: 300,
            },
            indexing: IndexingConfig {
                btree_node_size: 64,
                auto_index_primary_keys: true,
            },
            performance: PerformanceConfig {
                auto_compact: true,
                compact_threshold: 30,
                max_batch_size: 10000,
            },
            auth: AuthConfig {
                enabled: true,
                token_expiration: 86400,
                argon2_memory_cost: 65536,
                argon2_time_cost: 3,
                argon2_parallelism: 4,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                log_to_file: false,
                log_file: "./logs/rdb.log".to_string(),
            },
            limits: LimitsConfig {
                max_result_rows: 100000,
                max_query_time: 30,
                max_payload_size: 10485760,
            },
        }
    }
}

pub struct ConfigManager {
    pub root_dir: std::path::PathBuf,
    config_dir: std::path::PathBuf,
    data_dir: std::path::PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let root_dir = directories::ProjectDirs::from("com", "rdb", "rdb")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".rdb"));
        
        let config_dir = root_dir.join("config");
        let data_dir = root_dir.join("data");
        
        Ok(Self {
            root_dir,
            config_dir,
            data_dir,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        std::fs::create_dir_all(&self.root_dir)?;
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(self.root_dir.join("databases"))?;
        
        // Create default config.toml if it doesn't exist
        let config_path = self.config_path();
        if !config_path.exists() {
            let default_config = Config::default();
            self.save_config(&default_config)?;
        }
        
        // Also create a config.toml in the project root for convenience
        let project_config = std::path::PathBuf::from("config.toml");
        if !project_config.exists() {
            let default_config = Config::default();
            let content = toml::to_string_pretty(&default_config)?;
            std::fs::write(project_config, content)?;
        }
        
        Ok(())
    }

    pub fn config_path(&self) -> std::path::PathBuf {
        self.config_dir.join("config.toml")
    }

    #[allow(dead_code)]
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    pub fn get_database_path(&self, db_name: &str) -> std::path::PathBuf {
        self.root_dir.join("databases").join(format!("{}.db", db_name))
    }

    pub fn load_config(&self) -> Result<Config> {
        let path = self.config_path();
        if path.exists() {
            Config::load_from_file(path)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        let path = self.config_path();
        let content = toml::to_string_pretty(config)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
