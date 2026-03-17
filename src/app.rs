use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes};

use crate::fps::FpsCounter;
use crate::maths::vec3::Vec3;
use crate::scenes::scene::Scene;

pub struct App {
    window: Option<&'static dyn Window>,
    pixels: Option<Pixels<'static>>,
    scene: Scene,
    fps_counter: FpsCounter,
}

impl App {
    pub fn new(scene: Scene) -> Self {
        Self {
            window: None,
            pixels: None,
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
                self.scene.framebuffer.clear([0, 0, 0, 255]);
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
        let attrs = WindowAttributes::default()
            .with_title("rust-renderer")
            .with_surface_size(winit::dpi::PhysicalSize {
                width: self.scene.framebuffer.width as f32,
                height: self.scene.framebuffer.height as f32,
            });

        let window = event_loop.create_window(attrs).unwrap();

        // Leak the window to get a 'static reference
        let window_ref: &'static dyn Window = Box::leak(window);

        let window_size = window_ref.surface_size();

        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window_ref);
        let pixels = PixelsBuilder::new(window_size.width, window_size.height, surface_texture)
            .present_mode(pixels::wgpu::PresentMode::AutoNoVsync)
            .build()
            .unwrap();

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
                self.scene.framebuffer.clear([0, 0, 0, 255]);

                self.scene.render_objects();
                self.fps_counter.tick(&mut self.scene.framebuffer);

                let pixels = self.pixels.as_mut().unwrap();
                let bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        self.scene.framebuffer.pixels.as_ptr() as *const u8,
                        self.scene.framebuffer.pixels.len(),
                    )
                };
                pixels.frame_mut().copy_from_slice(bytes);

                pixels.render().unwrap();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                let pixels = self.pixels.as_mut().unwrap();
                if let Err(e) = pixels.resize_surface(new_size.width, new_size.height) {
                    eprintln!("Failed to resize surface: {:?}", e);
                }
                if let Err(e) = pixels.resize_buffer(new_size.width, new_size.height) {
                    eprintln!("Failed to resize buffer: {:?}", e);
                }
                self.scene
                    .framebuffer
                    .resize(new_size.width as usize, new_size.height as usize);
                self.scene.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;
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
