use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes};

use crate::framebuffer::Framebuffer;
use crate::shapes::Shape;
use crate::shapes::cube::Cube;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

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
            framebuffer: Framebuffer::new(WIDTH as usize, HEIGHT as usize),
        }
    }

    /// Handle keyboard entries
    /// This mainly exists as a helper to prevent the window_event function
    /// from becoming too large
    fn handle_keyboard(&mut self, key_event: &KeyEvent) {
        if key_event.state != ElementState::Pressed {
            return;
        }
        match &key_event.logical_key {
            Key::Character(ch) if ch == "c" => {
                self.framebuffer.clear([0, 0, 0, 255]);
            }
            Key::Character(ch) if ch == "d" => {
                let cube = Cube::new(1.0);
                cube.draw(&mut self.framebuffer);
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attrs = WindowAttributes::default().with_title("rust-renderer");

        let window = event_loop.create_window(attrs).unwrap();

        let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, window.as_ref());
        // extend lifetime
        let pixels = unsafe {
            std::mem::transmute::<Pixels<'_>, Pixels<'static>>(
                Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap(),
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
                pixels.frame_mut().copy_from_slice(&self.framebuffer.pixels);

                pixels.render().unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                self.handle_keyboard(&key_event);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.resumed(event_loop);
    }
}
