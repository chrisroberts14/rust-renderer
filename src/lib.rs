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

use geometry::obj_loader::ObjLoader;
use geometry::object::Object;
use geometry::transform::Transform;
use maths::vec3::Vec3;
use scenes::material::Material;
use scenes::pointlight::PointLight;
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

/// Creates a simple scene with one monkey and 3 lights
///
/// Returns a scene a handle for the object update thread and a bool for if that thread is running
pub fn create_simple_scene() -> Result<SceneCreateReturn, Box<dyn std::error::Error>> {
    let monkey = PathBuf::from("assets/monkey.obj");

    let scene_objects = vec![Object::new(
        ObjLoader::load(monkey.clone()),
        Transform::default(),
        Material::Color([255, 255, 255, 255]),
    )];

    let scene_lights = vec![
        PointLight::new(Vec3::new(0.0, 0.0, 5.0), [1.0, 0.0, 0.0], 15.0),
        PointLight::new(Vec3::new(5.0, 0.0, 0.0), [0.0, 1.0, 0.0], 20.0),
        PointLight::new(Vec3::new(-5.0, 0.0, 0.0), [0.0, 0.0, 1.0], 15.0),
    ];

    let scene = Scene::new(800.0, 600.0, scene_objects, scene_lights);

    let (update_handle, update_running) = scene.spawn_update_thread();

    Ok(SceneCreateReturn {
        scene,
        join_handle: update_handle,
        is_scene_update_thread_running: update_running,
    })
}

/// Creates a "complex" scene which involves 4 monkeys and 3 lights
pub fn create_complex_scene() -> Result<SceneCreateReturn, Box<dyn std::error::Error>> {
    let monkey = PathBuf::from("assets/monkey.obj");
    let scene_objects = vec![
        Object::new(
            ObjLoader::load(monkey.clone()),
            Transform::default(),
            Material::Color([255, 255, 255, 255]),
        ),
        Object::new(
            ObjLoader::load(monkey.clone()),
            Transform::in_position(Vec3::new(0.0, 5.0, 0.0)),
            Material::Color([255, 255, 255, 255]),
        ),
        Object::new(
            ObjLoader::load(monkey.clone()),
            Transform::in_position(Vec3::new(5.0, 0.0, 0.0)),
            Material::Color([255, 255, 255, 255]),
        ),
        Object::new(
            ObjLoader::load(monkey.clone()),
            Transform::in_position(Vec3::new(0.0, 0.0, -5.0)),
            Material::Color([255, 255, 255, 255]),
        ),
    ];

    let scene_lights = vec![
        PointLight::new(Vec3::new(0.0, 0.0, 5.0), [1.0, 0.0, 0.0], 15.0),
        PointLight::new(Vec3::new(5.0, 0.0, 0.0), [0.0, 1.0, 0.0], 20.0),
        PointLight::new(Vec3::new(-5.0, 0.0, 0.0), [0.0, 0.0, 1.0], 15.0),
    ];

    let scene = Scene::new(800.0, 600.0, scene_objects, scene_lights);

    let (update_handle, update_running) = scene.spawn_update_thread();

    Ok(SceneCreateReturn {
        scene,
        join_handle: update_handle,
        is_scene_update_thread_running: update_running,
    })
}

pub fn create_from_file() -> Result<SceneCreateReturn, Box<dyn std::error::Error>> {
    let scene = SceneFile::to_scene(PathBuf::from("assets/scene_defs/simple.json"))?;
    let (update_handle, update_running) = scene.spawn_update_thread();

    Ok(SceneCreateReturn {
        scene,
        join_handle: update_handle,
        is_scene_update_thread_running: update_running,
    })
}
