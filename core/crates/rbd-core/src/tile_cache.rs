use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Cache key identifying an individual rendered tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub doc_id: u64,
    pub page: u32,
    pub zoom_bucket: u32,
    pub tile_x: u32,
    pub tile_y: u32,
    pub pixel_format: u8,
}

/// Thread-safe LRU cache for rendered bitmap tiles.
pub struct TileCache {
    cache: Mutex<LruCache<TileKey, Arc<Vec<u8>>>>,
    capacity: usize,
}

impl TileCache {
    /// Create a new TileCache with the specified capacity (in tile count).
    /// Default standard tile size is typically 256x256x4 (256 KB).
    /// For example, 256 tiles ≈ 64 MB of cache.
    pub fn new(capacity_tiles: usize) -> Self {
        let cap = NonZeroUsize::new(capacity_tiles.max(1)).unwrap();
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            capacity: capacity_tiles,
        }
    }

    /// Retrieve a cached tile if available.
    pub fn get(&self, key: &TileKey) -> Option<Arc<Vec<u8>>> {
        let mut guard = self.cache.lock();
        guard.get(key).cloned()
    }

    /// Insert or update a rendered tile in the cache.
    pub fn insert(&self, key: TileKey, data: Arc<Vec<u8>>) {
        let mut guard = self.cache.lock();
        guard.put(key, data);
    }

    /// Clear all cached tiles.
    pub fn clear(&self) {
        let mut guard = self.cache.lock();
        guard.clear();
    }

    /// Invalidate all cached tiles for the specified page of a document.
    pub fn invalidate_page(&self, doc_id: u64, page: u32) {
        let mut guard = self.cache.lock();
        let keys_to_remove: Vec<TileKey> = guard
            .iter()
            .filter_map(|(k, _)| {
                if k.doc_id == doc_id && k.page == page {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect();

        for k in keys_to_remove {
            guard.pop(&k);
        }
    }

    /// Return the current number of cached tiles.
    pub fn len(&self) -> usize {
        let guard = self.cache.lock();
        guard.len()
    }

    /// Return whether the cache is currently empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the maximum tile capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for TileCache {
    fn default() -> Self {
        // Default capacity: 512 tiles ≈ 128 MB RAM for 256x256x4 tiles
        Self::new(512)
    }
}
