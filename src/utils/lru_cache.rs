//! LRU (Least Recently Used) Cache implementation
//!
//! Provides a fixed-size cache that automatically evicts the least recently used items
//! when capacity is reached.

use std::collections::{HashMap, VecDeque};

/// LRU Cache with fixed capacity
///
/// This cache automatically removes the least recently used entry when it reaches
/// capacity limits. It's designed for use cases like tracking file modifications
/// where we want to limit memory usage.
#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruCache<K, V> {
    /// Create a new LRU cache with the specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of items to store
    ///
    /// # Panics
    /// Panics if capacity is 0
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LRU cache capacity must be greater than 0");
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Create a new LRU cache with default capacity of 1000
    pub fn with_default_capacity() -> Self {
        Self::new(1000)
    }

    /// Insert a key-value pair into the cache
    ///
    /// If the key already exists, its value is updated and it becomes the most recently used.
    /// If the cache is at capacity, the least recently used item is evicted.
    ///
    /// # Returns
    /// * `Some(evicted_value)` - If an item was evicted
    /// * `None` - If no item was evicted
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // If key exists, update and move to front
        if self.map.contains_key(&key) {
            // Remove old value if it exists in order
            self.order.retain(|k| k != &key);
            self.order.push_front(key.clone());
            self.map.insert(key, value);
            return None;
        }

        // If at capacity, evict least recently used
        let evicted = if self.len() >= self.capacity {
            if let Some(oldest_key) = self.order.pop_back() {
                self.map.remove(&oldest_key)
            } else {
                None
            }
        } else {
            None
        };

        // Insert new item at front
        self.order.push_front(key.clone());
        self.map.insert(key, value);

        evicted
    }

    /// Get a value by key, marking it as most recently used
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move to front
            self.order.retain(|k| k != key);
            self.order.push_front(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    /// Get a value by key without updating LRU order
    pub fn get_peek(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    /// Remove a key-value pair from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|k| k != key);
        self.map.remove(key)
    }

    /// Check if the cache contains a key
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Get the current number of items in the cache
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Get the cache capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all items from the cache
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    /// Get the most recently used key
    pub fn most_recent(&self) -> Option<&K> {
        self.order.front()
    }

    /// Get the least recently used key
    pub fn least_recent(&self) -> Option<&K> {
        self.order.back()
    }

    /// Update the cache capacity
    ///
    /// If the new capacity is smaller than the current size, items will be evicted
    /// until the cache fits the new capacity.
    pub fn set_capacity(&mut self, new_capacity: usize) {
        assert!(new_capacity > 0, "LRU cache capacity must be greater than 0");
        
        // Evict items if necessary
        while self.len() > new_capacity {
            if let Some(oldest_key) = self.order.pop_back() {
                self.map.remove(&oldest_key);
            }
        }
        
        self.capacity = new_capacity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_creation() {
        let cache: LruCache<String, i32> = LruCache::new(10);
        assert_eq!(cache.capacity(), 10);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_insert_and_get() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        cache.insert("key3", 30);
        
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"key1"), Some(&10));
        assert_eq!(cache.get(&"key2"), Some(&20));
        assert_eq!(cache.get(&"key3"), Some(&30));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        cache.insert("key3", 30);
        
        // Access key1 to make it most recent
        cache.get(&"key1");
        
        // Insert key4, should evict key2 (least recently used)
        let evicted = cache.insert("key4", 40);
        
        assert_eq!(evicted, Some(20));
        assert_eq!(cache.len(), 3);
        assert!(cache.contains_key(&"key1"));
        assert!(!cache.contains_key(&"key2"));
        assert!(cache.contains_key(&"key3"));
        assert!(cache.contains_key(&"key4"));
    }

    #[test]
    fn test_lru_cache_update() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        
        // Update existing key
        cache.insert("key1", 100);
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"key1"), Some(&100));
    }

    #[test]
    fn test_lru_cache_remove() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        
        let removed = cache.remove(&"key1");
        
        assert_eq!(removed, Some(10));
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains_key(&"key1"));
        assert!(cache.contains_key(&"key2"));
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        
        cache.clear();
        
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_most_least_recent() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        cache.insert("key3", 30);
        
        // key1 is least recent, key3 is most recent
        assert_eq!(cache.most_recent(), Some(&"key3"));
        assert_eq!(cache.least_recent(), Some(&"key1"));
        
        // Access key1
        cache.get(&"key1");
        
        // Now key2 is least recent, key1 is most recent
        assert_eq!(cache.most_recent(), Some(&"key1"));
        assert_eq!(cache.least_recent(), Some(&"key2"));
    }

    #[test]
    fn test_lru_cache_set_capacity() {
        let mut cache = LruCache::new(5);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        cache.insert("key3", 30);
        
        assert_eq!(cache.len(), 3);
        
        // Reduce capacity to 2
        // LRU evicts from least recently used (oldest), so key1 will be evicted
        cache.set_capacity(2);
        
        assert_eq!(cache.capacity(), 2);
        assert_eq!(cache.len(), 2);
        // key2 and key3 should remain (key1 was evicted as least recently used)
        assert!(!cache.contains_key(&"key1"));
        assert!(cache.contains_key(&"key2"));
        assert!(cache.contains_key(&"key3"));
    }

    #[test]
    fn test_lru_cache_get_peek() {
        let mut cache = LruCache::new(3);
        
        cache.insert("key1", 10);
        cache.insert("key2", 20);
        
        // Peek without updating order
        assert_eq!(cache.get_peek(&"key1"), Some(&10));
        
        // key2 should still be most recent
        assert_eq!(cache.most_recent(), Some(&"key2"));
        
        // Now use regular get
        assert_eq!(cache.get(&"key1"), Some(&10));
        
        // key1 should now be most recent
        assert_eq!(cache.most_recent(), Some(&"key1"));
    }

    #[test]
    #[should_panic(expected = "LRU cache capacity must be greater than 0")]
    fn test_lru_cache_zero_capacity() {
        let _cache: LruCache<String, i32> = LruCache::new(0);
    }
}
