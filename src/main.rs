use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;

mod app;
mod framebuffer;
mod geometry;
mod maths;
mod renderer;
use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let app = App::new();

    event_loop.run_app(app)?;
    Ok(())
}
