use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::file::scene_file::SceneFile;
use crate::file::watcher::FileWatcher;
use crate::scenes::scene::Scene;

pub mod file_iter;
pub mod key_bindings_file;
pub mod scene_file;
mod watcher;

pub struct SceneFileWatcher {
    scene: Arc<Mutex<Scene>>,
    _watcher: FileWatcher,
}

impl SceneFileWatcher {
    pub fn new(path: PathBuf, width: f32, height: f32) -> Self {
        let scene = Arc::new(Mutex::new(
            SceneFile::from_file(path.clone(), width, height).unwrap(),
        ));
        let scene_clone = Arc::clone(&scene);
        let watcher = FileWatcher::new(
            path.clone(),
            Box::new(
                move || match SceneFile::from_file(path.clone(), width, height) {
                    Ok(mut new_scene) => {
                        if let Ok(mut lock) = scene_clone.lock() {
                            new_scene.settings = lock.settings.clone();
                            new_scene.camera = lock.camera.clone();
                            *lock = new_scene;
                        }
                    }
                    Err(e) => eprintln!("Failed to reload scene: {e}"),
                },
            ),
        );
        Self {
            scene,
            _watcher: watcher,
        }
    }

    pub fn scene(&self) -> MutexGuard<'_, Scene> {
        self.scene.lock().unwrap()
    }
}
