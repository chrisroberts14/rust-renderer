pub mod app;
pub mod fps;
pub mod framebuffer;
pub mod geometry;
pub mod maths;
pub mod renderer;
pub mod scenes;
pub mod tile;

use geometry::obj_loader::ObjLoader;
use geometry::object::Object;
use geometry::transform::Transform;
use maths::vec3::Vec3;
use scenes::material::Material;
use scenes::pointlight::PointLight;
use scenes::scene::Scene;
use scenes::texture::Texture;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

/// Creates a default scene
///
/// Returns a scene a handle for the object update thread and a bool for if that thread is running
pub fn create_scene() -> Result<(Scene, JoinHandle<()>, Arc<AtomicBool>), Box<dyn std::error::Error>>
{
    let monkey = Object::new(
        ObjLoader::load(Path::new("monkey.obj"))?,
        Transform::default(),
        Material::Color([255, 255, 255, 255]),
    );

    let scene_objects = vec![monkey];

    let scene_lights = vec![
        PointLight::new(Vec3::new(0.0, 0.0, 5.0), [1.0, 0.0, 0.0], 15.0),
        PointLight::new(Vec3::new(5.0, 0.0, 0.0), [0.0, 1.0, 0.0], 20.0),
        PointLight::new(Vec3::new(-5.0, 0.0, 0.0), [0.0, 0.0, 1.0], 15.0),
    ];

    let mut scene = Scene::new(800.0, 600.0, scene_objects, scene_lights);

    scene.skybox = Texture::load(Path::new("test.png")).ok();

    let (update_handle, update_running) = scene.spawn_update_thread();

    Ok((scene, update_handle, update_running))
}
