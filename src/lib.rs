pub mod app;
pub mod cache;
pub mod file;
pub mod fps;
pub mod framebuffer;
pub mod geometry;
pub mod maths;
pub mod renderer;
pub mod scenes;
pub mod tile;

pub use crate::cache::LruCache;
use crate::file::scene_file::SceneFile;

use scenes::scene::Scene;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

pub struct SceneCreateReturn {
    pub scene: Scene,
    pub join_handle: JoinHandle<()>,
    pub is_scene_update_thread_running: Arc<AtomicBool>,
}

pub fn create_from_file(
    file_path: PathBuf,
) -> Result<SceneCreateReturn, Box<dyn std::error::Error>> {
    let scene = SceneFile::to_scene(file_path)?;
    let (update_handle, update_running) = scene.spawn_update_thread();

    Ok(SceneCreateReturn {
        scene,
        join_handle: update_handle,
        is_scene_update_thread_running: update_running,
    })
}
