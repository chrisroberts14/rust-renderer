use winit::event_loop::{ControlFlow, EventLoop};

use rust_renderer::app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let app = App::new(None, 800.0, 600.0)?;

    event_loop.run_app(app)?;

    Ok(())
}
