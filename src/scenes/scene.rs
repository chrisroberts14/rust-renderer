use crate::geometry::cube::Cube;
use crate::geometry::transform::Transform;
use crate::geometry::update_thread::{UpdateThread, spawn_update_thread_for};
use crate::maths::vec3::Vec3;
use crate::renderer::{RenderStats, Renderer};
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use crate::scenes::scene_settings::SceneSettings;
use crate::scenes::texture::Texture;
use crate::{framebuffer::Framebuffer, geometry::object::Object};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Scene {
    objects: Arc<RwLock<Vec<Object>>>,
    pub(crate) framebuffer: Framebuffer,
    pub(crate) camera: Camera,
    lights: Vec<Arc<dyn Light>>,
    skybox: Option<Texture>,
    _update_thread: Option<UpdateThread>, // Exists solely so when it is dropped the thread is stopped cleanly
    pub(crate) settings: SceneSettings,
    renderer: Arc<dyn Renderer>,
    ambient: f32,
}

impl Scene {
    pub fn new(
        width: f32,
        height: f32,
        objects: Vec<Object>,
        lights: Vec<Arc<dyn Light>>,
        renderer: Arc<dyn Renderer>,
        ambient: f32,
    ) -> Self {
        let objects = Arc::new(RwLock::new(objects));
        let running = Arc::new(AtomicBool::new(true));

        Self {
            _update_thread: Some(spawn_update_thread_for(&objects, &running)),
            objects,
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            lights,
            settings: SceneSettings::new(),
            skybox: None,
            renderer,
            ambient,
        }
    }

    pub(crate) fn spawn_update_thread(&self) -> UpdateThread {
        let running = Arc::new(AtomicBool::new(true));
        spawn_update_thread_for(&self.objects, &running)
    }

    fn dispatch_render(&self, objects: &[Object], lights: &[Arc<dyn Light>]) -> RenderStats {
        match self.settings.wire_frame_mode {
            true => self
                .renderer
                .render_wireframe(objects, &self.camera, &self.framebuffer),
            false => self.renderer.render_objects(
                objects,
                &self.camera,
                lights,
                &self.framebuffer,
                self.ambient,
            ),
        }
    }

    /// Renders small box representations of each point light for debugging.
    /// Light boxes are rendered unlit so their colour always matches the light colour.
    pub fn render_lights(&mut self) {
        let light_objects: Vec<Object> = self
            .lights
            .iter()
            .map(|light| {
                let c = light.colour();
                let colour = [
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                    255,
                ];
                Object::new(
                    Cube::mesh(1.0),
                    Transform {
                        position: light.position(),
                        rotation: Vec3::ZERO,
                        scale: Vec3::new(0.1, 0.1, 0.1),
                    },
                    Material::Color(colour),
                )
            })
            .collect();

        // Pass empty lights — light boxes should appear unlit.
        self.dispatch_render(&light_objects, &[] as &[Arc<dyn Light>]);
    }

    /// Toggle rendering point lights as debug cubes
    pub fn toggle_render_lights(&mut self) {
        self.settings.toggle_render_lights();
    }

    /// Toggle wireframe rendering mode
    pub fn toggle_wire_frame_mode(&mut self) {
        self.settings.toggle_wire_frame_mode();
    }

    /// Helper method to render the whole scene
    pub fn render_scene(&mut self) -> RenderStats {
        self.framebuffer.clear();
        if let Some(skybox) = &self.skybox {
            self.framebuffer.draw_skybox(skybox, &self.camera);
        }
        if self.settings.render_lights {
            self.render_lights();
        }
        self.dispatch_render(
            &self.objects.read().unwrap_or_else(|e| e.into_inner()),
            &self.lights,
        )
    }
}
