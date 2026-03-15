use pixels::SurfaceTexture;
use winit::{event_loop::EventLoop};

mod framebuffer;
mod app;
//use framebuffer::Framebuffer;
use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let app= App::new();

    event_loop.run_app(app)?;
    Ok(())
}
