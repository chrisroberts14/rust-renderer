use std::iter::Cycle;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use std::vec::IntoIter;

use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes};

use crate::display::DisplaySurface;
use crate::file::key_bindings_file::{Action, KeyBindings};
use crate::file::scene_file::{SceneFile, get_all_scene_files};
use crate::fps::FpsCounter;
use crate::framebuffer::Framebuffer;
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
    fps_counter: FpsCounter,
    last_frame_time: Instant,
    fast_move: bool,
    scene_files: Option<Cycle<IntoIter<PathBuf>>>, // If this is empty a specific scene was rendered
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

        match scene_option {
            Some(scene) => Ok(Self {
                display: None,
                scene,
                fps_counter: FpsCounter::new(),
                last_frame_time: Instant::now(),
                fast_move: false,
                scene_files: None,
                renderer,
                overlays: OverlayManager::new(stats_overlay),
                key_bindings,
            }),
            _ => {
                let mut scene_files_iter = get_all_scene_files()?.into_iter().cycle();
                let next_scene = scene_files_iter.next().ok_or("No scene files found")?;
                let scene = SceneFile::from_file(next_scene, width, height)?;

                Ok(Self {
                    display: None,
                    scene,
                    fps_counter: FpsCounter::new(),
                    last_frame_time: Instant::now(),
                    fast_move: false,
                    scene_files: Some(scene_files_iter),
                    renderer,
                    overlays: OverlayManager::new(stats_overlay),
                    key_bindings,
                })
            }
        }
    }

    fn move_speed(&self) -> f32 {
        if self.fast_move {
            FAST_SPEED
        } else {
            NORMAL_SPEED
        }
    }

    fn perform_action(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::MoveForward => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * self.move_speed());
                Ok(())
            }
            Action::MoveBackward => {
                let dir = self.scene.camera.forward();
                self.scene.camera.move_camera(dir * -self.move_speed());
                Ok(())
            }
            Action::MoveRight => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * self.move_speed());
                Ok(())
            }
            Action::MoveLeft => {
                let dir = self.scene.camera.right();
                self.scene.camera.move_camera(dir * -self.move_speed());
                Ok(())
            }
            Action::MoveUp => {
                self.scene
                    .camera
                    .move_camera(self.scene.camera.up() * self.move_speed());
                Ok(())
            }
            Action::MoveDown => {
                self.scene
                    .camera
                    .move_camera(self.scene.camera.up() * -self.move_speed());
                Ok(())
            }
            Action::SpeedModifier => Ok(()),
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
                self.display.as_mut().unwrap().release_mouse()?;
                Ok(())
            }
            Action::NextRenderer => {
                let choice = self.renderer.renderer_choice().next();
                // Clear the overlay so only stats from the new renderer are shown
                self.overlays
                    .create_new_stats_overlay(vec![("renderer_type", &format!("{}", choice))]);
                self.renderer = if matches!(choice, RendererChoice::Gpu) {
                    if let Some(display) = &self.display {
                        Box::new(GpuRasterRenderer::from_display(display))
                    } else {
                        choice.into_renderer()
                    }
                } else {
                    choice.into_renderer()
                };

                Ok(())
            }
        }
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
        match action {
            Action::SpeedModifier => {
                self.fast_move = key_event.state == ElementState::Pressed;
                Ok(())
            }
            _ if key_event.state == ElementState::Pressed => self.perform_action(&action),
            _ => Ok(()),
        }
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
            self.renderer = Box::new(GpuRasterRenderer::from_display(
                self.display.as_ref().unwrap(),
            ));
        }

        self.display
            .as_mut()
            .unwrap()
            .capture_mouse()
            .expect("Failed to capture mouse");

        // Request the first frame to be drawn
        self.display
            .as_ref()
            .expect("Display not initialized")
            .request_redraw();
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
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_frame_time);
                self.last_frame_time = now;
                self.fps_counter.tick(elapsed);

                if self.scene.settings.show_overlay {
                    for (key, val) in &stats {
                        self.overlays.add_stat(key, val);
                    }
                    self.overlays
                        .add_stat("fps", &self.fps_counter.fps.to_string());
                }

                let display = self.display.as_ref().expect("Display not initialized");
                if let Some(view) = self.renderer.take_gpu_view() {
                    let overlay = self.scene.settings.show_overlay.then(|| {
                        let mut fb = Framebuffer::new(
                            self.scene.framebuffer.width,
                            self.scene.framebuffer.height,
                        );
                        self.overlays.write_to_framebuffer(&mut fb);
                        fb
                    });
                    display.present_gpu_frame(&view, overlay.as_ref().map(|fb| fb.as_bytes()));
                } else {
                    if self.scene.settings.show_overlay {
                        self.overlays
                            .write_to_framebuffer(&mut self.scene.framebuffer);
                    }
                    display.present_cpu_frame(self.scene.framebuffer.as_bytes());
                }

                // Render the next frame
                self.display
                    .as_ref()
                    .expect("Display not initialized")
                    .request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                self.display
                    .as_mut()
                    .expect("Display not initialized")
                    .resize(new_size.width, new_size.height);
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
                if gained_focus && let Err(error) = self.display.as_mut().unwrap().capture_mouse() {
                    eprintln!("Error locking the mouse: {:?}", error);
                }
            }
            WindowEvent::PointerButton {
                state: ElementState::Pressed,
                ..
            } => self
                .display
                .as_mut()
                .unwrap()
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
            && self.display.as_ref().unwrap().is_cursor_grabbed()
        {
            self.scene.camera.process_mouse(dx as f32, dy as f32);
        }
    }
}
