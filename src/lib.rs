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
use crate::renderer::multi_thread_raster_renderer::MultiThreadRasterRenderer;

use crate::renderer::Renderer;
use scenes::scene::Scene;
use std::path::PathBuf;
use std::sync::Arc;

/// This is a thin wrapper around `SceneFile::from_file` to create a `Scene` from a file path.
/// It's mainly used for benchmarking purposes.
pub fn create_from_file(
    file_path: PathBuf,
    renderer: Option<Arc<dyn Renderer>>,
) -> Result<Scene, Box<dyn std::error::Error>> {
    match renderer {
        Some(renderer) => SceneFile::from_file(file_path, 800.0, 600.0, renderer),
        None => SceneFile::from_file(file_path, 800.0, 600.0, Arc::new(MultiThreadRasterRenderer)),
    }
}
