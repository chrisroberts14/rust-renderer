use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes};

use crate::display::Display;
use crate::file::SceneFileWatcher;
use crate::file::file_iter::FileIter;
use crate::file::key_bindings_file::{Action, KeyBindings};
use crate::maths::vec3::Vec3;
use crate::renderer::ActiveRenderer;
use crate::renderer::cpu::display::CpuDisplay;
use crate::renderer::vulkan::VulkanRenderer;
use crate::renderer::vulkan::display::VulkanDisplay;
use crate::renderer::wgsl::WGSLRenderer;
use crate::renderer::wgsl::display::WgslDisplay;
use crate::terminal::StatsDisplay;

const KEYBINDINGS_PATH: &str = "assets/keybindings.json";
const NORMAL_SPEED: f32 = 0.05;
const FAST_SPEED: f32 = 0.25;

pub struct App {
    display: Option<Box<dyn Display>>,
    scene: SceneFileWatcher,
    fast_move: bool,
    scene_files: Option<FileIter>,
    renderer: ActiveRenderer,
    stats_display: StatsDisplay,
    key_bindings: KeyBindings,
}

impl App {
    pub fn new(
        scene_option: Option<SceneFileWatcher>,
        renderer: ActiveRenderer,
        width: f32,
        height: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key_bindings = KeyBindings::from_file_or_default(KEYBINDINGS_PATH);

        let (scene, scene_files) = if let Some(scene) = scene_option {
            (scene, None)
        } else {
            let mut iter = FileIter::with_extension("assets/scene_defs", "json")?;
            let next = iter.next().ok_or("No scene files found")?;
            let scene = SceneFileWatcher::new(next, width, height);
            (scene, Some(iter))
        };

        Ok(Self {
            display: None,
            scene,
            fast_move: false,
            scene_files,
            renderer,
            stats_display: StatsDisplay::new(),
            key_bindings,
        })
    }

    /// Creates the appropriate display backend for the current renderer, initialising a GPU
    /// renderer against the new display's shared device/queue if needed.
    fn make_display(
        renderer: &mut ActiveRenderer,
        window: Arc<dyn Window>,
        width: u32,
        height: u32,
    ) -> Box<dyn Display> {
        match renderer {
            ActiveRenderer::Gpu(_) => {
                let wgsl = WgslDisplay::new(window, width as usize, height as usize);
                *renderer = ActiveRenderer::Gpu(Box::new(WGSLRenderer::from_display(&wgsl)));
                Box::new(wgsl)
            }
            ActiveRenderer::Vulkan(_) => {
                let vulkan = VulkanDisplay::new(window, width, height);
                *renderer = ActiveRenderer::Vulkan(Box::new(
                    VulkanRenderer::from_display(&vulkan)
                        .expect("Failed to create Vulkan renderer"),
                ));
                Box::new(vulkan)
            }
            _ => Box::new(CpuDisplay::new(window, width, height)),
        }
    }

    fn move_camera(&mut self, direction: Vec3, sign: f32) {
        let speed = if self.fast_move {
            FAST_SPEED
        } else {
            NORMAL_SPEED
        };
        let mut scene = self.scene.scene();
        let new_position = scene.camera.position + (direction * sign * speed);
        if !scene.is_point_inside_any_object(&new_position) {
            scene.camera.position = new_position;
        }
    }

    fn display_mut(&mut self) -> &mut dyn Display {
        self.display
            .as_deref_mut()
            .expect("Display not initialized")
    }

    fn display_ref(&self) -> &dyn Display {
        self.display.as_deref().expect("Display not initialized")
    }

    fn perform_action(&mut self, action: &Action) -> Result<(), Box<dyn std::error::Error>> {
        let (forward, right, up) = {
            let scene = self.scene.scene();
            (
                scene.camera.forward(),
                scene.camera.right(),
                scene.camera.up(),
            )
        };
        match action {
            Action::MoveForward => self.move_camera(forward, 1.0),
            Action::MoveBackward => self.move_camera(forward, -1.0),
            Action::MoveRight => self.move_camera(right, 1.0),
            Action::MoveLeft => self.move_camera(right, -1.0),
            Action::MoveUp => self.move_camera(up, 1.0),
            Action::MoveDown => self.move_camera(up, -1.0),
            Action::ToggleWireframe => {
                self.scene.scene().settings.toggle_wire_frame_mode();
            }
            Action::ToggleLights => {
                self.scene.scene().settings.toggle_render_lights();
            }
            Action::NextScene => {
                if let Some(next_scene) = self.scene_files.as_mut().and_then(|sf| sf.next()) {
                    let (old_settings, w, h) = {
                        let scene = self.scene.scene();
                        (
                            scene.settings.clone(),
                            scene.framebuffer.width as f32,
                            scene.framebuffer.height as f32,
                        )
                    };
                    self.scene = SceneFileWatcher::new(next_scene, w, h);
                    self.scene.scene().settings = old_settings;
                }
            }
            Action::IncreaseTiles => {
                self.renderer.increase_tile_count(1);
            }
            Action::DecreaseTiles => {
                self.renderer.decrease_tile_count(1);
            }
            Action::ReleaseMouse => {
                self.display_mut().release_mouse()?;
            }
            Action::NextRenderer => {
                let window = self.display_ref().window();
                let width = self.scene.scene().framebuffer.width as u32;
                let height = self.scene.scene().framebuffer.height as u32;

                // Drop the current display before creating the new one. The two wgpu stacks
                // (wgpu 29 for WgslDisplay, wgpu 27 inside pixels for CpuDisplay) can't both
                // hold an active D3D12 swap chain for the same HWND simultaneously.
                self.display = None;

                self.renderer.next();
                self.display = Some(Self::make_display(
                    &mut self.renderer,
                    window,
                    width,
                    height,
                ));
            }
            _ => {}
        }
        Ok(())
    }

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

fn named_key_to_str(key: &NamedKey) -> Option<&'static str> {
    match key {
        NamedKey::Shift => Some("shift"),
        NamedKey::Control => Some("ctrl"),
        NamedKey::Escape => Some("escape"),
        _ => None,
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        let (fb_width, fb_height) = {
            let scene = self.scene.scene();
            (
                scene.framebuffer.width as f32,
                scene.framebuffer.height as f32,
            )
        };
        let attrs = WindowAttributes::default()
            .with_title("rust-renderer")
            .with_surface_size(winit::dpi::PhysicalSize {
                width: fb_width,
                height: fb_height,
            });

        let window: Arc<dyn Window> = event_loop
            .create_window(attrs)
            .expect("Failed to create window")
            .into();

        let window_size = window.surface_size();

        self.display = Some(Self::make_display(
            &mut self.renderer,
            window,
            window_size.width,
            window_size.height,
        ));

        self.display_mut()
            .capture_mouse()
            .expect("Failed to capture mouse");

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
                let (stats, settings) = {
                    let mut scene = self.scene.scene();
                    let stats = scene.render_scene(&self.renderer);
                    let settings = scene.settings.as_pairs();
                    (stats, settings)
                };

                let lines: Vec<String> = std::iter::once(format!("renderer: {}", self.renderer))
                    .chain(stats.into_iter().map(|(k, v)| format!("{}: {}", k, v)))
                    .chain(settings.into_iter().map(|(k, v)| format!("{}: {}", k, v)))
                    .collect();

                self.stats_display.update(lines);

                if let Some(view) = self.renderer.take_gpu_view() {
                    self.display_ref().present_gpu_frame(&view, None);
                } else if let Some(image) = self.renderer.take_vk_image() {
                    self.display_ref().present_vk_frame(&image, None);
                } else {
                    let scene = self.scene.scene();
                    self.display_ref()
                        .present_cpu_frame(scene.framebuffer.as_bytes());
                }

                self.display_ref().request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                self.display_mut().resize(new_size.width, new_size.height);
                let mut scene = self.scene.scene();
                scene
                    .framebuffer
                    .resize(new_size.width as usize, new_size.height as usize);
                scene.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if let Err(error) = self.handle_keyboard(&key_event) {
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
            self.scene
                .scene()
                .camera
                .process_mouse(dx as f32, dy as f32);
        }
    }
}
