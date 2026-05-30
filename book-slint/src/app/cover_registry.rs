use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

type Key = (String, String);

const COVER_REGISTRY_MAX: usize = 1024;

#[derive(Default)]
struct Registry {
    map: HashMap<Key, (Vec<u8>, u32, u32)>,
    order: VecDeque<Key>, // front = least-recent, back = most-recent
}

impl Registry {
    fn insert(&mut self, key: Key, value: (Vec<u8>, u32, u32)) {
        // update map
        self.map.insert(key.clone(), value);
        // mark recent
        self.order.push_back(key);
        // evict if over capacity
        while self.map.len() > COVER_REGISTRY_MAX {
            if let Some(old) = self.order.pop_front() {
                // remove stale key if present
                self.map.remove(&old);
            } else {
                break;
            }
        }
    }

    fn get(&mut self, key: &Key) -> Option<(Vec<u8>, u32, u32)> {
        if let Some(v) = self.map.get(key) {
            // mark recent by pushing key to back (duplicates in order are acceptable)
            self.order.push_back(key.clone());
            return Some(v.clone());
        }
        None
    }
}

// Store raw RGBA bytes and dimensions. `Vec<u8>` is Send + Sync-friendly, unlike `slint::Image`.
static COVER_REGISTRY: Lazy<Arc<RwLock<Registry>>> = Lazy::new(|| Arc::new(RwLock::new(Registry::default())));

pub fn insert(source_id: &str, book_id: &str, rgba: Vec<u8>, width: u32, height: u32) {
    let key = (source_id.to_string(), book_id.to_string());
    let registry: Arc<RwLock<Registry>> = Arc::clone(&COVER_REGISTRY);
    let mut reg = registry.write().unwrap();
    reg.insert(key, (rgba, width, height));
}

pub fn get(source_id: &str, book_id: &str) -> Option<(Vec<u8>, u32, u32)> {
    let key = (source_id.to_string(), book_id.to_string());
    let registry: Arc<RwLock<Registry>> = Arc::clone(&COVER_REGISTRY);
    // Use write lock so we can update recency ordering on access
    let mut reg = registry.write().unwrap();
    reg.get(&key)
}

