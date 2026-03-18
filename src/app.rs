use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes};

use crate::fps::FpsCounter;
use crate::scenes::scene::Scene;

pub struct App {
    window: Option<&'static dyn Window>,
    pixels: Option<Pixels<'static>>,
    scene: Scene,
    fps_counter: FpsCounter,
    cursor_grabbed: bool,
}

impl App {
    pub fn new(scene: Scene) -> Self {
        Self {
            window: None,
            pixels: None,
            scene,
            fps_counter: FpsCounter::new(),
            cursor_grabbed: false,
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
            Key::Character(ch) if ch == "w" => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * 0.05);
            }
            Key::Character(ch) if ch == "s" => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * -0.05);
            }
            Key::Character(ch) if ch == "d" => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * 0.05);
            }
            Key::Character(ch) if ch == "a" => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * -0.05);
            }
            Key::Character(ch) if ch == " " => {
                self.scene.camera.move_camera(self.scene.camera.up() * 0.05);
            }
            Key::Named(NamedKey::Shift) => {
                self.scene
                    .camera
                    .move_camera(self.scene.camera.up() * -0.05);
            }
            Key::Named(NamedKey::Escape) => {
                self.window.unwrap().set_cursor_visible(true);
                self.window
                    .unwrap()
                    .set_cursor_grab(CursorGrabMode::None)
                    .unwrap();
                self.cursor_grabbed = false;
            }
            _ => {}
        }
    }

    /// Lock the mouse to the window and hide the cursor
    /// With some window managers we have to use Locked and for others (like Wayland) we have to
    /// use Confined, so we try Locked first and fall back to Confined
    fn lock_mouse(&mut self) {
        if let Some(window) = self.window {
            window.set_cursor_visible(false);
            if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
            }
            self.cursor_grabbed = true;
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

        // Leak the window to get a static reference
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

        self.lock_mouse();

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
                self.scene.render_lights();
                self.fps_counter.tick(&mut self.scene.framebuffer);

                let pixels = self.pixels.as_mut().unwrap();
                pixels
                    .frame_mut()
                    .copy_from_slice(self.scene.framebuffer.as_bytes());

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
            WindowEvent::Focused(gained_focus) => {
                if gained_focus {
                    self.lock_mouse();
                } else {
                    self.cursor_grabbed = false;
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                ..
            } if !self.cursor_grabbed => {
                self.lock_mouse();
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &dyn ActiveEventLoop,
        _device_id: Option<DeviceId>,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::PointerMotion { delta: (dx, dy) } = event
            && self.cursor_grabbed
        {
            self.scene.camera.process_mouse(dx as f32, dy as f32);
        }
    }
}
