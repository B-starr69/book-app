use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

type Key = (String, String);

// Store raw RGBA bytes and dimensions. `Vec<u8>` is Send + Sync-friendly, unlike `slint::Image`.
static COVER_REGISTRY: Lazy<Arc<RwLock<HashMap<Key, (Vec<u8>, u32, u32)>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn insert(source_id: &str, book_id: &str, rgba: Vec<u8>, width: u32, height: u32) {
    let key = (source_id.to_string(), book_id.to_string());
    let registry: Arc<RwLock<HashMap<Key, (Vec<u8>, u32, u32)>>> = Arc::clone(&COVER_REGISTRY);
    let mut map = registry.write().unwrap();
    map.insert(key, (rgba, width, height));
}

pub fn get(source_id: &str, book_id: &str) -> Option<(Vec<u8>, u32, u32)> {
    let key = (source_id.to_string(), book_id.to_string());
    let registry: Arc<RwLock<HashMap<Key, (Vec<u8>, u32, u32)>>> = Arc::clone(&COVER_REGISTRY);
    let map = registry.read().unwrap();
    map.get(&key).cloned()
}

pub fn registry_arc() -> Arc<RwLock<HashMap<Key, (Vec<u8>, u32, u32)>>> {
    Arc::clone(&COVER_REGISTRY)
}
