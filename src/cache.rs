/// Self implemented LRU cache for storing the results of functions
/// Takes in the arguments as a key and the result as a value, and stores them in a HashMap
/// When the cache reaches its capacity, it will remove the least recently used item
///
/// This implementation is not inherently thread safe so if you desire to use it across multiple threads mutexes/locks will be required
///
/// The initial implementation also used retain which is an O(n) operation which isn't ideal for large caches
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LRU Cache of a given size
#[allow(dead_code)]
pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

#[allow(dead_code)]
impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Get an item from the cache by it's key
    ///
    /// If it the key exists in the cache return the item and move the key to the front of the order
    /// (last to be removed)
    ///
    /// If it doesn't exist return None
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // This removes the key from the order
            self.order.retain(|k| k != key);
            // Push it to the front
            self.order.push_front(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    /// Inset a value into the cache
    ///
    /// If the cache is full remove the last used item and insert this one
    ///
    /// If the key already exists it is updated
    pub fn insert(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // This removes the key from the order so later when we push to the front it's the same as updating
            self.order.retain(|k| k != &key);
        } else if self.map.len() == self.capacity {
            // Remove the least recently used item
            if let Some(key_to_remove) = self.order.pop_back() {
                self.map.remove(&key_to_remove);
            }
        }

        self.order.push_front(key.clone());
        self.map.insert(key, value);
    }
}

/// Macro definition of cache
///
/// Allows the use of cached! around a function which will mean any call to the function will run through an LRU cache first
#[macro_export]
macro_rules! cached {
    (
        $vis:vis fn $name:ident($arg:ident : $key_ty:ty) -> $ret:ty = $cap:expr => $body:block
    ) => {
        $vis fn $name($arg: $key_ty) -> $ret {
            use std::sync::{Mutex, OnceLock};

            static CACHE: OnceLock<Mutex<$crate::LruCache<$key_ty, $ret>>> = OnceLock::new();

            let cache = CACHE.get_or_init(|| Mutex::new($crate::LruCache::new($cap)));

            let mut cache_guard = cache.lock().unwrap();

            if let Some(v) = cache_guard.get(&$arg) {
                return v.clone();
            }

            // drop the read lock before computing
            drop(cache_guard);
            let result = {
                $body
            };

            // lock again to insert
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert($arg.clone(), result.clone());

            result
        }
    };
}
