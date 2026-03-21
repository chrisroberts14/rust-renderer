use rust_renderer::create_simple_scene;
use winit::event_loop::{ControlFlow, EventLoop};

use rust_renderer::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let (scene, update_handle, update_running) = create_simple_scene()?;

    let app = App::new(scene);

    event_loop.run_app(app)?;

    update_running.store(false, std::sync::atomic::Ordering::Relaxed);
    update_handle.join().unwrap();

    Ok(())
}
