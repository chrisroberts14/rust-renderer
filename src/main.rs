use clap::Parser;
use winit::event_loop::{ControlFlow, EventLoop};

use rust_renderer::app::App;
use rust_renderer::renderer::RendererChoice;

#[derive(Parser)]
struct Args {
    /// Which renderer to use
    #[arg(long, value_enum, default_value_t = RendererChoice::MultiThreadRaster)]
    renderer: RendererChoice,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let renderer = args.renderer.into_renderer();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let app = App::new(None, renderer, 800.0, 600.0)?;

    event_loop.run_app(app)?;

    Ok(())
}
