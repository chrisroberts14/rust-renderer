use clap::Parser;
use winit::event_loop::{ControlFlow, EventLoop};

use rust_renderer::app::App;
use rust_renderer::renderer::RendererChoice;

#[derive(Parser)]
struct Args {
    /// Which renderer to use
    #[arg(long, value_enum, default_value_t = RendererChoice::Gpu)]
    renderer: RendererChoice,

    /// Width of the window to create
    #[arg(long, default_value_t = 800.0)]
    width: f32,

    /// Height of the window to create
    #[arg(long, default_value_t = 600.0)]
    height: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let renderer = args.renderer.into_renderer();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let app = App::new(None, renderer, args.width, args.height)?;

    event_loop.run_app(app)?;

    Ok(())
}
