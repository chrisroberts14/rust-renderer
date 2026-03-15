use winit::event_loop::{ActiveEventLoop};
use winit::window::{Window, WindowAttributes};
use winit::event::WindowEvent;
use winit::application::ApplicationHandler;
use pixels::{Pixels, SurfaceTexture};

use crate::framebuffer::Framebuffer;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;


#[derive(Default)]
pub struct App {
    window: Option<Box<dyn Window>>,
    pixels: Option<Pixels<'static>>,
    framebuffer: Framebuffer,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            framebuffer: Framebuffer::new(WIDTH as usize, HEIGHT as usize)
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("rust-renderer");

        let window = event_loop.create_window(attrs).unwrap();

        let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, window.as_ref());
        // extend lifetime
        let pixels = unsafe {
            std::mem::transmute::<Pixels<'_>, Pixels<'static>>(
                Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
            )
        };

        self.pixels = Some(pixels);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {

                let pixels = self.pixels.as_mut().unwrap();
                let frame = pixels.frame_mut();

                for pixel in frame.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&[255, 0, 0, 255]);
                }

                pixels.render().unwrap();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            _ => ()
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.resumed(event_loop);
    }
}
