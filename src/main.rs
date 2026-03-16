use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;

mod app;
mod framebuffer;
mod geometry;
mod maths;
mod renderer;
mod scenes;
use crate::geometry::cube::Cube;
use crate::geometry::object::Object;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::pointlight::PointLight;
use crate::scenes::scene::Scene;
use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut scene = Scene::new(app::HEIGHT as f32, app::WIDTH as f32);
    scene.add_object(Object {
        mesh: Cube::mesh(1.0),
        transform: Transform {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
        },
    });
    scene.add_object(Object {
        mesh: Cube::mesh(1.0),
        transform: Transform {
            position: Vec3::new(2.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
        },
    });
    scene.add_object(Object {
        mesh: Cube::mesh(1.0),
        transform: Transform {
            position: Vec3::new(-2.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
        },
    });

    scene.light = Some(PointLight::new(
        Vec3::new(0.0, 0.0, 1.0),
        [255.0, 255.0, 255.0],
        2.0,
    ));

    let app = App::new(scene);

    event_loop.run_app(app)?;
    Ok(())
}
