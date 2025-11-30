use actix_web::{web, HttpResponse};
use crate::core::config::Config;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

#[allow(dead_code)]
pub struct ConfigService {
    config: Arc<RwLock<Config>>,
}

#[allow(dead_code)]
impl ConfigService {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub fn get_config(&self) -> Config {
        self.config.read().clone()
    }

    pub fn update_config(&self, new_config: Config) {
        *self.config.write() = new_config;
    }

    pub fn update_partial(&self, updates: ConfigUpdate) {
        let mut config = self.config.write();
        
        if let Some(buffer_size) = updates.buffer_pool_size {
            config.storage.buffer_pool_size = buffer_size;
        }
        if let Some(cache_size) = updates.query_cache_size {
            config.cache.query_cache_size = cache_size;
        }
        if let Some(port) = updates.port {
            config.server.port = port;
        }
        if let Some(host) = updates.host {
            config.server.host = host;
        }
        if let Some(enabled) = updates.enable_cache {
            config.cache.enable_query_cache = enabled;
        }
        if let Some(auto_compact) = updates.auto_compact {
            config.performance.auto_compact = auto_compact;
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigUpdate {
    pub buffer_pool_size: Option<usize>,
    pub query_cache_size: Option<usize>,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub enable_cache: Option<bool>,
    pub auto_compact: Option<bool>,
}

// API Handlers
#[allow(dead_code)]
pub async fn get_config_handler(service: web::Data<ConfigService>) -> HttpResponse {
    let config = service.get_config();
    HttpResponse::Ok().json(config)
}

#[allow(dead_code)]
pub async fn update_config_handler(
    service: web::Data<ConfigService>,
    updates: web::Json<ConfigUpdate>,
) -> HttpResponse {
    service.update_partial(updates.into_inner());
    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "Configuration updated"
    }))
}

#[allow(dead_code)]
pub async fn reload_config_handler(
    service: web::Data<ConfigService>,
    config_path: web::Data<String>,
) -> HttpResponse {
    match Config::load_from_file(&**config_path) {
        Ok(new_config) => {
            service.update_config(new_config);
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Configuration reloaded from file"
            }))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to reload config: {}", e)
            }))
        }
    }
}
