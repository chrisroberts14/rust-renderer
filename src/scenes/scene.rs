use crate::geometry::cube::Cube;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::renderer::Renderer;
use crate::scenes::camera::Camera;
use crate::scenes::material::Material;
use crate::scenes::pointlight::PointLight;
use crate::scenes::texture::Texture;
use crate::{framebuffer::Framebuffer, geometry::object::Object};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone)]
pub struct SceneSettings {
    pub render_lights: bool,
    pub wire_frame_mode: bool,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneSettings {
    pub fn new() -> Self {
        Self {
            render_lights: false,
            wire_frame_mode: false,
        }
    }

    pub fn toggle_render_lights(&mut self) {
        self.render_lights = !self.render_lights;
    }

    pub fn toggle_wire_frame_mode(&mut self) {
        self.wire_frame_mode = !self.wire_frame_mode;
    }
}

/// Struct to return when creating the update thread
///
/// This exists so we can define a method that stops the thread cleanly when it is dropped
pub struct UpdateThread {
    join_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

/// This custom clone implementation doesn't actually copy anything it solely is used for simplicity
/// in benchmarks
impl Clone for UpdateThread {
    fn clone(&self) -> Self {
        Self {
            join_handle: None,
            running: self.running.clone(),
        }
    }
}

impl Drop for UpdateThread {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join(); // ignore join errors during drop
        }
    }
}

#[derive(Clone)]
pub struct Scene {
    pub objects: Arc<RwLock<Vec<Object>>>,
    pub framebuffer: Framebuffer,
    pub camera: Camera,
    pub lights: Vec<PointLight>,
    pub settings: SceneSettings,
    pub skybox: Option<Texture>,
    pub update_thread: Option<UpdateThread>,
    renderer: Arc<dyn Renderer>,
}

impl Scene {
    pub fn new(
        width: f32,
        height: f32,
        objects: Vec<Object>,
        lights: Vec<PointLight>,
        renderer: Arc<dyn Renderer>,
    ) -> Self {
        let objects = Arc::new(RwLock::new(objects));
        let running = Arc::new(AtomicBool::new(true));

        Self {
            update_thread: Some(Self::spawn_update_thread_for(&objects, &running)),
            objects,
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            lights,
            settings: SceneSettings::new(),
            skybox: None,
            renderer,
        }
    }

    /// Spawn a thread that continuously updates object transforms.
    /// Returns the join handle and a shutdown flag — set the flag to false and join the handle to stop the thread cleanly.
    fn spawn_update_thread_for(
        objects: &Arc<RwLock<Vec<Object>>>,
        running: &Arc<AtomicBool>,
    ) -> UpdateThread {
        let objects = Arc::clone(objects);
        let thread_running = Arc::clone(running);
        let handle = thread::spawn(move || {
            while thread_running.load(Ordering::Relaxed) {
                {
                    let mut objs = objects.write().unwrap();
                    for object in objs.iter_mut() {
                        object.transform.rotation.x =
                            (object.transform.rotation.x + 0.01) % (2.0 * std::f32::consts::PI);
                        object.transform.rotation.y =
                            (object.transform.rotation.y + 0.01) % (2.0 * std::f32::consts::PI);
                    }
                }
                thread::sleep(Duration::from_millis(16));
            }
        });
        UpdateThread {
            join_handle: Some(handle),
            running: Arc::clone(running),
        }
    }

    pub fn spawn_update_thread(&self) -> UpdateThread {
        let running = Arc::new(AtomicBool::new(true));
        Self::spawn_update_thread_for(&self.objects, &running)
    }

    pub fn render_objects(&mut self) {
        let objects = self.objects.read().unwrap();
        if self.settings.wire_frame_mode {
            self.renderer
                .render_wireframe(&objects, &self.camera, &self.framebuffer);
        } else {
            self.renderer
                .render_objects(&objects, &self.camera, &self.lights, &self.framebuffer);
        }
    }

    /// Renders small box representations of each point light for debugging.
    /// Light boxes are rendered unlit so their colour always matches the light colour.
    pub fn render_lights(&mut self) {
        let light_objects: Vec<Object> = self
            .lights
            .iter()
            .map(|light| {
                let colour = [
                    (light.colour[0] * 255.0) as u8,
                    (light.colour[1] * 255.0) as u8,
                    (light.colour[2] * 255.0) as u8,
                    255,
                ];
                Object::new(
                    Cube::mesh(1.0),
                    Transform {
                        position: light.position,
                        rotation: Vec3::new(0.0, 0.0, 0.0),
                        scale: Vec3::new(0.1, 0.1, 0.1),
                    },
                    Material::Color(colour),
                )
            })
            .collect();

        if self.settings.wire_frame_mode {
            self.renderer
                .render_wireframe(&light_objects, &self.camera, &self.framebuffer);
        } else {
            // Pass empty lights — light boxes should appear unlit.
            self.renderer
                .render_objects(&light_objects, &self.camera, &[], &self.framebuffer);
        }
    }

    /// Helper method to render the whole scene
    pub fn render_scene(&mut self) {
        self.framebuffer.clear();
        if let Some(skybox) = &self.skybox {
            self.framebuffer.draw_skybox(skybox, &self.camera);
        }
        self.render_objects();
        if self.settings.render_lights {
            self.render_lights();
        }
    }
}
