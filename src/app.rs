use std::iter::Cycle;
use std::path::PathBuf;
use std::vec::IntoIter;

use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes};

use crate::file::key_bindings_file::{Action, KeyBindings};
use crate::file::scene_file::{SceneFile, get_all_scene_files};
use crate::fps::FpsCounter;
use crate::overlay::StatsOverlay;
use crate::renderer::Renderer;
use crate::scenes::scene::Scene;

const KEYBINDINGS_PATH: &str = "assets/keybindings.json";

pub struct App {
    window: Option<&'static dyn Window>,
    pixels: Option<Pixels<'static>>,
    scene: Scene,
    fps_counter: FpsCounter,
    cursor_grabbed: bool,
    scene_files: Option<Cycle<IntoIter<PathBuf>>>, // If this is empty a specific scene was rendered
    renderer: Box<dyn Renderer>,
    overlay: StatsOverlay,
    key_bindings: KeyBindings,
}

impl App {
    /// Create the app with an optional Scene to render
    ///
    /// If this is left empty the first in the scene defs file will be loaded instead
    pub fn new(
        scene_option: Option<Scene>,
        renderer: Box<dyn Renderer>,
        width: f32,
        height: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key_bindings = KeyBindings::from_file_or_default(KEYBINDINGS_PATH);
        match scene_option {
            Some(scene) => Ok(Self {
                window: None,
                pixels: None,
                scene,
                fps_counter: FpsCounter::new(),
                cursor_grabbed: false,
                scene_files: None,
                renderer,
                overlay: StatsOverlay::new(),
                key_bindings,
            }),
            _ => {
                let mut scene_files_iter = get_all_scene_files()?.into_iter().cycle();
                let next_scene = scene_files_iter.next().ok_or("No scene files found")?;
                let scene = SceneFile::from_file(next_scene, width, height)?;

                Ok(Self {
                    window: None,
                    pixels: None,
                    scene,
                    fps_counter: FpsCounter::new(),
                    cursor_grabbed: false,
                    scene_files: Some(scene_files_iter),
                    renderer,
                    overlay: StatsOverlay::new(),
                    key_bindings,
                })
            }
        }
    }

    fn perform_action(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::MoveForward => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * 0.05);
                Ok(())
            }
            Action::MoveBackward => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * -0.05);
                Ok(())
            }
            Action::MoveRight => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * 0.05);
                Ok(())
            }
            Action::MoveLeft => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * -0.05);
                Ok(())
            }
            Action::MoveUp => {
                self.scene.camera.move_camera(self.scene.camera.up() * 0.05);
                Ok(())
            }
            Action::MoveDown => {
                self.scene
                    .camera
                    .move_camera(self.scene.camera.up() * -0.05);
                Ok(())
            }
            Action::ToggleWireframe => {
                self.scene.settings.toggle_wire_frame_mode();
                Ok(())
            }
            Action::ToggleLights => {
                self.scene.settings.toggle_render_lights();
                Ok(())
            }
            Action::NextScene => {
                if let Some(next_scene) = self.scene_files.as_mut().and_then(|sf| sf.next()) {
                    let old_settings = self.scene.settings.clone();
                    let scene = SceneFile::from_file(
                        next_scene,
                        self.scene.framebuffer.width as f32,
                        self.scene.framebuffer.height as f32,
                    )?;
                    self.scene = scene;
                    self.scene.settings = old_settings;
                    self.scene.spawn_update_thread();
                }
                Ok(())
            }
            Action::IncreaseTiles => {
                self.renderer.increase_tile_count(1);
                Ok(())
            }
            Action::DecreaseTiles => {
                self.renderer.decrease_tile_count(1);
                Ok(())
            }
            Action::ToggleOverlay => {
                self.scene.settings.toggle_overlay();
                Ok(())
            }
            Action::ReleaseMouse => {
                if let Some(window) = self.window {
                    window.set_cursor_visible(true);
                    window.set_cursor_grab(CursorGrabMode::None)?;
                    self.cursor_grabbed = false;
                    Ok(())
                } else {
                    Err("Window not initialized".into())
                }
            }
        }
    }

    /// Handle keyboard entries
    /// This mainly exists as a helper to prevent the window_event function
    /// from becoming too large
    fn handle_keyboard(&mut self, key_event: &KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        if key_event.state != ElementState::Pressed {
            return Ok(());
        }
        let key_str = match &key_event.logical_key {
            Key::Character(ch) => ch.to_string(),
            Key::Named(named_key) => match named_key_to_str(named_key) {
                Some(s) => s.to_string(),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        if let Some(action) = self.key_bindings.bindings.get(&key_str).cloned() {
            self.perform_action(&action)
        } else {
            Ok(())
        }
    }

    /// Lock the mouse to the window and hide the cursor
    /// With some window managers we have to use Locked and for others (like Wayland) we have to
    /// use Confined, so we try Locked first and fall back to Confined
    fn lock_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(window) = self.window {
            window.set_cursor_visible(false);
            if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                window.set_cursor_grab(CursorGrabMode::Confined)?;
            }
            self.cursor_grabbed = true;
        }
        Ok(())
    }
}

/// Map a winit `NamedKey` to the lowercase string used in the keybindings file
fn named_key_to_str(key: &NamedKey) -> Option<&'static str> {
    match key {
        NamedKey::Shift => Some("shift"),
        NamedKey::F1 => Some("f1"),
        NamedKey::Escape => Some("escape"),
        _ => None,
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

        let window = event_loop
            .create_window(attrs)
            .expect("Failed to create window");

        // Leak the window to get a static reference
        let window_ref: &'static dyn Window = Box::leak(window);

        let window_size = window_ref.surface_size();

        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window_ref);
        let pixels = PixelsBuilder::new(window_size.width, window_size.height, surface_texture)
            .present_mode(pixels::wgpu::PresentMode::AutoNoVsync)
            .build()
            .expect("Failed to create pixel buffer");

        self.window = Some(window_ref);
        self.pixels = Some(pixels);

        self.lock_mouse().expect("Failed to lock mouse on resume");

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
                // Render the whole scene and when that is done tick the fps counter
                let stats = self.scene.render_scene(&*self.renderer);
                self.fps_counter.tick(&mut self.overlay);

                if self.scene.settings.show_overlay {
                    // Write the render stats to the overlay
                    self.overlay
                        .add("Triangle Count", &stats.triangle_count.to_string());
                    self.overlay
                        .add("Tile Count", &stats.tile_count.to_string());
                    self.overlay
                        .write_to_framebuffer(&mut self.scene.framebuffer);
                }

                // Copy the newly generated frame into the pixel array which is what will be put on the screen
                let pixels = self.pixels.as_mut().expect("Pixels not initialized");
                pixels
                    .frame_mut()
                    .copy_from_slice(self.scene.framebuffer.as_bytes());
                pixels.render().expect("Failed to render frame");

                // Render the next frame
                self.window
                    .as_ref()
                    .expect("Window failed to initialise")
                    .request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                let pixels = self.pixels.as_mut().expect("Pixels not initialized");
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
                if let Err(error) = self.handle_keyboard(&key_event) {
                    // Log any keyboard errors
                    eprintln!("{:?}", error);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Focused(gained_focus) => {
                if gained_focus {
                    if let Err(error) = self.lock_mouse() {
                        eprintln!("Error locking the mouse: {:?}", error);
                    }
                } else {
                    self.cursor_grabbed = false;
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                ..
            } if !self.cursor_grabbed => {
                if let Err(error) = self.lock_mouse() {
                    eprintln!("Error locking the mouse: {:?}", error);
                }
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
