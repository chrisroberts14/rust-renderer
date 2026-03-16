use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes};

use crate::fps::FpsCounter;
use crate::framebuffer::Framebuffer;
use crate::maths::vec3::Vec3;
use crate::scenes::scene::Scene;

pub(crate) const WIDTH: u32 = 800;
pub(crate) const HEIGHT: u32 = 600;

pub struct App {
    window: Option<&'static dyn Window>,
    pixels: Option<Pixels<'static>>,
    framebuffer: Framebuffer,
    scene: Scene,
    fps_counter: FpsCounter,
}

impl App {
    pub fn new(scene: Scene) -> Self {
        Self {
            window: None,
            pixels: None,
            framebuffer: Framebuffer::new(WIDTH as usize, HEIGHT as usize),
            scene,
            fps_counter: FpsCounter::new(),
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
            Key::Character(ch) if ch == "w" => {
                // Move the camera forward
                self.scene.camera.move_camera(Vec3::new(0.0, 0.0, -0.05));
            }
            Key::Character(ch) if ch == "s" => {
                // Move the camera forward
                self.scene.camera.move_camera(Vec3::new(0.0, 0.0, 0.05));
            }
            Key::Character(ch) if ch == "d" => {
                // Move the camera forward
                self.scene.camera.move_camera(Vec3::new(0.05, 0.0, 0.0));
            }
            Key::Character(ch) if ch == "a" => {
                // Move the camera forward
                self.scene.camera.move_camera(Vec3::new(-0.05, 0.0, 0.0));
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attrs = WindowAttributes::default().with_title("rust-renderer");

        let window = event_loop.create_window(attrs).unwrap();

        // Leak the window to get a 'static reference
        let window_ref: &'static dyn Window = Box::leak(window);

        let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, window_ref);
        // extend lifetime
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();

        self.window = Some(window_ref);
        self.pixels = Some(pixels);

        window_ref.request_redraw();
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                // Clear the current framebuffer
                self.framebuffer.clear([0, 0, 0, 255]);

                self.scene.render_objects(&mut self.framebuffer);

                let pixels = self.pixels.as_mut().unwrap();
                pixels.frame_mut().copy_from_slice(&self.framebuffer.pixels);

                pixels.render().unwrap();
                self.fps_counter.tick();
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
}
