use rust_renderer::create_simple_scene;
use winit::event_loop::{ControlFlow, EventLoop};

use rust_renderer::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let scene_create_return = create_simple_scene()?;

    let app = App::new(scene_create_return.scene);

    event_loop.run_app(app)?;

    scene_create_return
        .is_scene_update_thread_running
        .store(false, std::sync::atomic::Ordering::Relaxed);
    scene_create_return.join_handle.join().unwrap();

    Ok(())
}
