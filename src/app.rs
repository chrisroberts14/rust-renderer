use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes};

use crate::display::DisplaySurface;
use crate::file::file_iter::FileIter;
use crate::file::key_bindings_file::{Action, KeyBindings};
use crate::file::scene_file::SceneFile;
use crate::framebuffer::Framebuffer;
use crate::maths::vec3::Vec3;
use crate::overlay::OverlayManager;
use crate::overlay::stats_overlay::StatsOverlay;
use crate::renderer::Renderer;
use crate::renderer::RendererChoice;
use crate::renderer::gpu_raster_renderer::GpuRasterRenderer;
use crate::scenes::scene::Scene;

const KEYBINDINGS_PATH: &str = "assets/keybindings.json";
const NORMAL_SPEED: f32 = 0.05;
const FAST_SPEED: f32 = 0.25;

pub struct App {
    display: Option<DisplaySurface<'static>>,
    scene: Scene,
    fast_move: bool,
    scene_files: Option<FileIter>, // If this is empty a specific scene was rendered
    renderer: Box<dyn Renderer>,
    overlays: OverlayManager,
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

        let renderer_choice = renderer.renderer_choice();
        let stats_overlay =
            StatsOverlay::with_defaults(vec![("renderer_type", &format!("{}", renderer_choice))]);

        let (scene, scene_files) = if let Some(scene) = scene_option {
            (scene, None)
        } else {
            let mut iter = FileIter::new("assets/scene_defs")?;
            let next = iter.next().ok_or("No scene files found")?;
            let scene = SceneFile::from_file(next, width, height)?;
            (scene, Some(iter))
        };

        Ok(Self {
            display: None,
            scene,
            fast_move: false,
            scene_files,
            renderer,
            overlays: OverlayManager::new(stats_overlay),
            key_bindings,
        })
    }

    fn move_camera(&mut self, direction: Vec3, sign: f32) {
        let speed = if self.fast_move {
            FAST_SPEED
        } else {
            NORMAL_SPEED
        };
        let new_position = self.scene.camera.position + (direction * sign * speed);
        // Check for collision with any objects
        if !self.scene.is_point_inside_any_object(&new_position) {
            self.scene.camera.move_camera(direction * speed * sign);
        }
    }

    fn display_mut(&mut self) -> &mut DisplaySurface<'static> {
        self.display.as_mut().expect("Display not initialized")
    }

    fn display_ref(&self) -> &DisplaySurface<'static> {
        self.display.as_ref().expect("Display not initialized")
    }

    fn perform_action(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::MoveForward => {
                self.move_camera(self.scene.camera.forward(), 1.0);
            }
            Action::MoveBackward => {
                self.move_camera(self.scene.camera.forward(), -1.0);
            }
            Action::MoveRight => {
                self.move_camera(self.scene.camera.right(), 1.0);
            }
            Action::MoveLeft => {
                self.move_camera(self.scene.camera.right(), -1.0);
            }
            Action::MoveUp => {
                self.move_camera(self.scene.camera.up(), 1.0);
            }
            Action::MoveDown => {
                self.move_camera(self.scene.camera.up(), -1.0);
            }
            Action::ToggleWireframe => {
                self.scene.settings.toggle_wire_frame_mode();
            }
            Action::ToggleLights => {
                self.scene.settings.toggle_render_lights();
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
            }
            Action::IncreaseTiles => {
                self.renderer.increase_tile_count(1);
            }
            Action::DecreaseTiles => {
                self.renderer.decrease_tile_count(1);
            }
            Action::ToggleOverlay => {
                self.scene.settings.toggle_overlay();
            }
            Action::ReleaseMouse => {
                self.display_mut().release_mouse()?;
            }
            Action::NextRenderer => {
                let choice = self.renderer.renderer_choice().next();
                // Clear the overlay so only stats from the new renderer are shown
                self.overlays
                    .create_new_stats_overlay(vec![("renderer_type", &format!("{}", choice))]);
                self.renderer = match choice {
                    RendererChoice::Gpu => {
                        if let Some(display) = &self.display {
                            Box::new(GpuRasterRenderer::from_display(display))
                        } else {
                            choice.into_renderer()
                        }
                    }
                    _ => choice.into_renderer(),
                };
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keyboard entries
    /// This mainly exists as a helper to prevent the window_event function
    /// from becoming too large
    fn handle_keyboard(&mut self, key_event: &KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        let key_str = match &key_event.logical_key {
            Key::Character(ch) => ch.to_string(),
            Key::Named(named_key) => match named_key_to_str(named_key) {
                Some(s) => s.to_string(),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let Some(action) = self.key_bindings.bindings.get(&key_str).cloned() else {
            return Ok(());
        };

        if matches!(action, Action::SpeedModifier) {
            self.fast_move = key_event.state == ElementState::Pressed;
            return Ok(());
        }

        if key_event.state == ElementState::Pressed {
            self.perform_action(&action)?;
        }

        Ok(())
    }
}

/// Map a winit `NamedKey` to the lowercase string used in the keybindings file
fn named_key_to_str(key: &NamedKey) -> Option<&'static str> {
    match key {
        NamedKey::Shift => Some("shift"),
        NamedKey::Control => Some("ctrl"),
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

        let window: Arc<dyn Window> = event_loop
            .create_window(attrs)
            .expect("Failed to create window")
            .into();

        let window_size = window.surface_size();

        let display = DisplaySurface::new(
            window,
            window_size.width as usize,
            window_size.height as usize,
        );

        self.display = Some(display);

        // If the renderer is GPU, reinitialize it using the shared device from the display so that
        // GPU textures can be blitted directly to the surface without a CPU readback.
        if self.renderer.renderer_choice() == RendererChoice::Gpu {
            self.renderer = Box::new(GpuRasterRenderer::from_display(self.display_ref()));
        }

        self.display_mut()
            .capture_mouse()
            .expect("Failed to capture mouse");

        // Request the first frame to be drawn
        self.display_ref().request_redraw();
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

                if self.scene.settings.show_overlay {
                    for (key, val) in &stats {
                        self.overlays.add_stat(key, val);
                    }
                    for (key, val) in self.scene.settings.as_pairs() {
                        self.overlays.add_stat(&key, &val);
                    }
                }

                if let Some(view) = self.renderer.take_gpu_view() {
                    let overlay = self.scene.settings.show_overlay.then(|| {
                        let mut fb = Framebuffer::new(
                            self.scene.framebuffer.width,
                            self.scene.framebuffer.height,
                        );
                        self.overlays.write_to_framebuffer(&mut fb);
                        fb
                    });
                    self.display_ref()
                        .present_gpu_frame(&view, overlay.as_ref().map(|fb| fb.as_bytes()));
                } else {
                    if self.scene.settings.show_overlay {
                        self.overlays
                            .write_to_framebuffer(&mut self.scene.framebuffer);
                    }
                    self.display_ref()
                        .present_cpu_frame(self.scene.framebuffer.as_bytes());
                }

                // Render the next frame
                self.display_ref().request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                self.display_mut().resize(new_size.width, new_size.height);
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
                if gained_focus && let Err(error) = self.display_mut().capture_mouse() {
                    eprintln!("Error locking the mouse: {:?}", error);
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                ..
            } => self
                .display_mut()
                .capture_mouse()
                .expect("Failed to capture mouse"),
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
            && self.display_ref().is_cursor_grabbed()
        {
            self.scene.camera.process_mouse(dx as f32, dy as f32);
        }
    }
}
