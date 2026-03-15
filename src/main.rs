use winit::{event_loop::EventLoop};

//mod framebuffer;
mod app;
//use framebuffer::Framebuffer;
use app::App;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let app= App::default();
    event_loop.run_app(app)?;
    Ok(())
}
