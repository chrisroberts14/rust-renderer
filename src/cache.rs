/// Self implemented LRU cache for storing the results of functions
/// Takes in the arguments as a key and the result as a value, and stores them in a HashMap
/// When the cache reaches its capacity, it will remove the least recently used item
///
/// This implementation is not inherently thread safe so if you desire to use it across multiple threads mutexes/locks will be required
///
/// The initial implementation also used retain which is an O(n) operation which isn't ideal for large caches
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// LRU Cache of a given size
pub struct LruCache<K: Eq + Hash + Clone, V: Clone> {
    capacity: usize,
    map: HashMap<K, Arc<Mutex<Node<K, V>>>>,
    head: Link<K, V>, // Most recent
    tail: Link<K, V>, // Least recent
}

type Link<K, V> = Option<Arc<Mutex<Node<K, V>>>>;

struct Node<K: Eq + Hash + Clone, V: Clone> {
    key: K,
    value: V,
    prev: Link<K, V>,
    next: Link<K, V>,
}

impl<K: Eq + Hash + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            head: None,
            tail: None,
        }
    }

    /// Get an item from the cache by it's key
    ///
    /// If the key exists in the cache return the item and move the key to the front of the order
    /// (last to be removed)
    ///
    /// If it doesn't exist return None
    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(node) = self.map.get(key).cloned() {
            self.move_to_front(node.clone());
            Some(node.lock().unwrap().value.clone())
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
        if let Some(existing) = self.map.get(&key).cloned() {
            existing.lock().unwrap().value = value;
            self.move_to_front(existing);
            return;
        }

        let node = Arc::new(Mutex::new(Node {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        }));

        self.push_front(node.clone());
        self.map.insert(key, node);

        if self.map.len() > self.capacity
            && let Some(old_tail) = self.tail.clone()
        {
            let key = old_tail.lock().unwrap().key.clone();
            self.remove(old_tail);
            self.map.remove(&key);
        }
    }

    /// Remove a node from the cache
    fn remove(&mut self, node: Arc<Mutex<Node<K, V>>>) {
        let (prev, next) = {
            let n = node.lock().unwrap();
            (n.prev.clone(), n.next.clone())
        };

        if let Some(ref p) = prev {
            p.lock().unwrap().next = next.clone();
        } else {
            self.head = next.clone();
        }

        if let Some(ref n) = next {
            n.lock().unwrap().prev = prev.clone();
        } else {
            self.tail = prev.clone();
        }
    }

    /// Add a node to the front of the cache
    fn push_front(&mut self, node: Arc<Mutex<Node<K, V>>>) {
        {
            let mut n = node.lock().unwrap();
            n.prev = None;
            n.next = self.head.clone();
        }

        if let Some(ref head) = self.head {
            head.lock().unwrap().prev = Some(node.clone());
        }

        self.head = Some(node.clone());

        if self.tail.is_none() {
            self.tail = Some(node);
        }
    }

    /// Move a node to the front of the cache
    fn move_to_front(&mut self, node: Arc<Mutex<Node<K, V>>>) {
        self.remove(node.clone());
        self.push_front(node);
    }
}

/// Macro definition of cache
///
/// Allows the use of cached! around a function which will mean any call to the function will run through an LRU cache first
#[macro_export]
macro_rules! cached {
    (
        $vis:vis fn $name:ident($($arg:ident : $arg_ty:ty),+) -> Result<$ok:ty, $err:ty> = $cap:expr => $body:block
    ) => {
        $vis fn $name($($arg: $arg_ty),+) -> Result<$ok, $err> {
            use std::sync::{Mutex, OnceLock};

            type CacheKey = ($($arg_ty,)+);
            static CACHE: OnceLock<Mutex<$crate::LruCache<CacheKey, $ok>>> = OnceLock::new();

            let cache = CACHE.get_or_init(|| Mutex::new($crate::LruCache::new($cap)));
            let key: CacheKey = ($($arg.clone(),)+);

            {
                let mut cache_guard = cache.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(v) = cache_guard.get(&key) {
                    return Ok(v);
                }
            } // lock released here

            let result: Result<$ok, $err> = { $body };

            if let Ok(ref value) = result {
                let mut cache_guard = cache.lock().unwrap_or_else(|e| e.into_inner());
                cache_guard.insert(key, value.clone());
            }

            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_with_missing_key() {
        let mut cache = LruCache::<i32, i32>::new(10);
        assert_eq!(cache.get(&1), None);
    }

    #[test]
    fn test_get_previously_inserted_value() {
        let mut cache = LruCache::<i32, i32>::new(10);
        cache.insert(1, 2);
        assert_eq!(cache.get(&1), Some(2));
    }

    #[test]
    fn test_inserting_same_value_updates() {
        let mut cache = LruCache::<i32, i32>::new(10);
        cache.insert(1, 2);
        assert_eq!(cache.get(&1), Some(2));
        cache.insert(1, 3);
        assert_eq!(cache.get(&1), Some(3));
    }
}
