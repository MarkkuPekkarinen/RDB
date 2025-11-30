use parking_lot::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(dead_code)]
pub struct QueryCache {
    cache: RwLock<LruCache<u64, Value>>,
}

#[allow(dead_code)]
impl QueryCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("Capacity must be > 0");
        Self {
            cache: RwLock::new(LruCache::new(cap)),
        }
    }

    pub fn get(&self, query_json: &str) -> Option<Value> {
        let key = Self::hash_query(query_json);
        let mut cache = self.cache.write();
        cache.get(&key).cloned()
    }

    pub fn put(&self, query_json: &str, result: Value) {
        let key = Self::hash_query(query_json);
        let mut cache = self.cache.write();
        cache.put(key, result);
    }

    pub fn invalidate_all(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    pub fn invalidate_table(&self, _table: &str) {
        // For simplicity, invalidate all cache on any table modification
        // In production, we'd track which queries touch which tables
        self.invalidate_all();
    }

    fn hash_query(query_json: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        query_json.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = QueryCache::new(10);
        let query = r#"{"Select":{"from":"users"}}"#;
        let result = serde_json::json!(["data"]);

        cache.put(query, result.clone());
        let cached = cache.get(query);

        assert_eq!(cached, Some(result));
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = QueryCache::new(10);
        let query = r#"{"Select":{"from":"users"}}"#;
        let result = serde_json::json!(["data"]);

        cache.put(query, result);
        cache.invalidate_all();

        assert_eq!(cache.get(query), None);
    }
}
