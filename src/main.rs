use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;

mod app;
mod fps;
mod framebuffer;
mod geometry;
mod maths;
mod renderer;
mod scenes;
mod texture;

use crate::geometry::obj_loader::ObjLoader;
use crate::geometry::object::Object;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::pointlight::PointLight;
use crate::scenes::scene::Scene;
use crate::texture::Texture;
use app::App;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let monkey = Object::new(
        ObjLoader::load(Path::new("monkey.obj"), [255, 255, 255, 255])?,
        Transform::default(),
    );

    let scene_objects = vec![monkey];

    let scene_lights = vec![
        PointLight::new(Vec3::new(0.0, 0.0, 5.0), [1.0, 0.0, 0.0], 15.0),
        PointLight::new(Vec3::new(5.0, 0.0, 0.0), [0.0, 1.0, 0.0], 20.0),
        PointLight::new(Vec3::new(-5.0, 0.0, 0.0), [0.0, 0.0, 1.0], 15.0),
    ];

    let mut scene = Scene::new(800.0, 600.0, scene_objects, scene_lights);

    scene.skybox = Some(Texture::load(Path::new("ticknock_04_4k.exr"))?);

    let (update_handle, update_running) = scene.spawn_update_thread();

    let app = App::new(scene);

    event_loop.run_app(app)?;

    update_running.store(false, std::sync::atomic::Ordering::Relaxed);
    update_handle.join().unwrap();

    Ok(())
}
