use once_cell::sync::Lazy;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;

type Key = (String, String);

const COVER_REGISTRY_MAX: usize = 1024;

// Store raw RGBA bytes and dimensions in an LRU cache. Values are `Arc<Vec<u8>>`
// so clones are cheap (atomic refcount increments) and we avoid copying buffers
// on each `get`.
static COVER_REGISTRY: Lazy<Mutex<LruCache<Key, (Arc<Vec<u8>>, u32, u32)>>> = Lazy::new(|| {
    let cap = NonZeroUsize::new(COVER_REGISTRY_MAX).unwrap();
    Mutex::new(LruCache::new(cap))
});

pub fn insert(source_id: &str, book_id: &str, rgba: Arc<Vec<u8>>, width: u32, height: u32) {
    let key = (source_id.to_string(), book_id.to_string());
    let mut cache = COVER_REGISTRY.lock();
    cache.put(key, (rgba, width, height));
}

pub fn get(source_id: &str, book_id: &str) -> Option<(Arc<Vec<u8>>, u32, u32)> {
    let key = (source_id.to_string(), book_id.to_string());
    // LruCache::get mutates internal ordering, so we need a write lock.
    let mut cache = COVER_REGISTRY.lock();
    if let Some(v) = cache.get(&key) {
        // clone the Arc (cheap)
        return Some((v.0.clone(), v.1, v.2));
    }
    None
}

