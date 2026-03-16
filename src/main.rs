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

    let mut scene = Scene::new(app::HEIGHT as f32, app::WIDTH as f32);
    let monkey_mesh = ObjLoader::load(Path::new("monkey.obj"), [255, 255, 0, 255])?;
    let teapot_mesh = ObjLoader::load(Path::new("teapot.obj"), [255, 0, 255, 255])?;
    scene.add_object(Object::new(monkey_mesh, Transform::new()));
    scene.add_object(Object::new(teapot_mesh, Transform::new()));

    scene.light = Some(PointLight::new(
        Vec3::new(0.0, 0.0, 5.0),
        [255.0, 255.0, 255.0],
        15.0,
    ));

    let app = App::new(scene);

    event_loop.run_app(app)?;
    Ok(())
}
