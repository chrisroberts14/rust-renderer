use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;

mod app;
mod fps;
mod framebuffer;
mod geometry;
mod maths;
mod renderer;
mod scenes;
use crate::geometry::obj_loader::ObjLoader;
use crate::geometry::object::Object;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::pointlight::PointLight;
use crate::scenes::scene::Scene;
use app::App;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    // Here we create the scene
    // For now this consists of a teapot a monkey and 3 point lights of different colours

    let scene_objects = vec![
        Object::new(
            ObjLoader::load(Path::new("monkey.obj"), [255, 255, 255, 255])?,
            Transform::with_position(Vec3::new(0.0, 2.0, 0.0)),
        ),
        Object::new(
            ObjLoader::load(Path::new("teapot.obj"), [255, 255, 255, 255])?,
            Transform::with_position(Vec3::new(0.0, -2.0, 0.0)),
        ),
    ];

    let scene_lights = vec![
        PointLight::new(Vec3::new(0.0, 0.0, 5.0), [1.0, 0.0, 0.0], 15.0),
        PointLight::new(Vec3::new(5.0, 0.0, 0.0), [0.0, 1.0, 0.0], 20.0),
        PointLight::new(Vec3::new(-5.0, 0.0, 0.0), [0.0, 0.0, 1.0], 15.0),
    ];

    let scene = Scene::new(800.0, 600.0, scene_objects, scene_lights);

    let _update_handle = scene.spawn_update_thread();

    let app = App::new(scene);

    event_loop.run_app(app)?;
    Ok(())
}
