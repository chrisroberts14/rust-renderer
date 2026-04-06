use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::geometry::object::Object;

/// Struct to return when creating the update thread
///
/// This exists so we can define a method that stops the thread cleanly when it is dropped
#[derive(Debug)]
pub struct UpdateThread {
    join_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

/// This custom clone implementation doesn't actually copy anything it solely is used for simplicity
/// in benchmarks
impl Clone for UpdateThread {
    fn clone(&self) -> Self {
        Self {
            join_handle: None,
            running: self.running.clone(),
        }
    }
}

impl Drop for UpdateThread {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join(); // ignore join errors during drop
        }
    }
}

/// Controls how a scene's objects are updated each tick.
pub trait UpdateStrategy {
    /// Attach the strategy to the object list. Returns an `UpdateThread` handle if a background
    /// thread was spawned; `None` for strategies that don't run a thread.
    fn start(self, objects: &Arc<RwLock<Vec<Object>>>) -> Option<UpdateThread>;
}

/// Spawns a background thread that calls each object's update function every ~16 ms.
pub struct ThreadedUpdate;

impl UpdateStrategy for ThreadedUpdate {
    fn start(self, objects: &Arc<RwLock<Vec<Object>>>) -> Option<UpdateThread> {
        let running = Arc::new(AtomicBool::new(true));
        Some(spawn_update_thread_for(objects, &running))
    }
}

/// Does not run any updates. Useful in tests where deterministic, animation-free scenes are needed.
pub struct NoOpUpdate;

impl UpdateStrategy for NoOpUpdate {
    fn start(self, _objects: &Arc<RwLock<Vec<Object>>>) -> Option<UpdateThread> {
        None
    }
}

fn spawn_update_thread_for(
    objects: &Arc<RwLock<Vec<Object>>>,
    running: &Arc<AtomicBool>,
) -> UpdateThread {
    let objects = Arc::clone(objects);
    let thread_running = Arc::clone(running);
    let handle = thread::spawn(move || {
        while thread_running.load(Ordering::Relaxed) {
            {
                let mut objs = objects.write().unwrap_or_else(|e| e.into_inner());
                for object in objs.iter_mut() {
                    object.update();
                }
            }
            thread::sleep(Duration::from_millis(16));
        }
    });
    UpdateThread {
        join_handle: Some(handle),
        running: Arc::clone(running),
    }
}
