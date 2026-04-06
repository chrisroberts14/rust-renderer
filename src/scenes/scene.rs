use crate::geometry::cube::Cube;
use crate::geometry::transform::Transform;
use crate::geometry::update_thread::{UpdateThread, spawn_update_thread_for};
use crate::maths::vec3::Vec3;
use crate::renderer::Renderer;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use crate::scenes::scene_settings::SceneSettings;
use crate::scenes::texture::Texture;
use crate::{framebuffer::Framebuffer, geometry::object::Object};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct Scene {
    objects: Arc<RwLock<Vec<Object>>>,
    pub(crate) framebuffer: Framebuffer,
    pub(crate) camera: Camera,
    lights: Vec<Arc<dyn Light>>,
    skybox: Option<Texture>,
    _update_thread: Option<UpdateThread>, // Exists solely so when it is dropped the thread is stopped cleanly
    pub settings: SceneSettings,
    ambient: f32,
}

impl Scene {
    pub fn new(
        width: f32,
        height: f32,
        objects: Vec<Object>,
        lights: Vec<Arc<dyn Light>>,
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
            settings: SceneSettings::default(),
            skybox: None,
            ambient,
        }
    }

    pub(crate) fn spawn_update_thread(&self) -> UpdateThread {
        let running = Arc::new(AtomicBool::new(true));
        spawn_update_thread_for(&self.objects, &running)
    }

    pub(crate) fn is_point_inside_any_object(&self, point: &Vec3) -> bool {
        self.objects
            .read()
            .unwrap()
            .iter()
            .any(|obj| obj.is_within_bounding_box(point))
    }

    /// If the camera is inside any object's bounding box (e.g. because the object moved into it),
    /// push the camera out along the axis of minimum penetration.
    fn push_camera_out_of_objects(&mut self) {
        let bounding_boxes: Vec<(Vec3, Vec3)> = {
            let objects = self.objects.read().unwrap();
            objects
                .iter()
                .filter_map(|obj| obj.bounding_box())
                .collect()
        };

        for (min, max) in bounding_boxes {
            let p = self.camera.position;
            if p.x < min.x
                || p.x > max.x
                || p.y < min.y
                || p.y > max.y
                || p.z < min.z
                || p.z > max.z
            {
                continue;
            }
            // Find the face with the smallest overlap and push the camera out through it.
            let overlaps = [
                (p.x - min.x, Vec3::new(-1.0, 0.0, 0.0)),
                (max.x - p.x, Vec3::new(1.0, 0.0, 0.0)),
                (p.y - min.y, Vec3::new(0.0, -1.0, 0.0)),
                (max.y - p.y, Vec3::new(0.0, 1.0, 0.0)),
                (p.z - min.z, Vec3::new(0.0, 0.0, -1.0)),
                (max.z - p.z, Vec3::new(0.0, 0.0, 1.0)),
            ];
            let (depth, axis) = overlaps
                .iter()
                .copied()
                .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap();
            self.camera.position = self.camera.position + (axis * depth);
        }
    }

    fn dispatch_render(
        &self,
        renderer: &dyn Renderer,
        objects: &[Object],
        lights: &[Arc<dyn Light>],
    ) -> Vec<(&'static str, String)> {
        if self.settings.wire_frame_mode {
            renderer.render_wireframe(objects, &self.camera, &self.framebuffer)
        } else {
            renderer.render_objects(
                objects,
                &self.camera,
                lights,
                &self.framebuffer,
                self.ambient,
            )
        }
    }

    /// Builds small box representations of each light source for debug rendering.
    fn build_light_objects(&self) -> Vec<Object> {
        self.lights
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
                .as_light()
            })
            .collect()
    }

    /// Helper method to render the whole scene
    ///
    /// Clears the framebuffer then does the following:
    /// 1. Draw the skybox
    /// 2. Render the scene objects (with light source boxes appended, if enabled)
    pub fn render_scene(&mut self, renderer: &dyn Renderer) -> Vec<(&'static str, String)> {
        self.push_camera_out_of_objects();
        self.framebuffer.clear();
        if let Some(skybox) = &self.skybox {
            self.framebuffer.draw_skybox(skybox, &self.camera);
        }
        let objects_guard = self.objects.read().unwrap_or_else(|e| e.into_inner());
        let objects: Vec<Object> = if self.settings.render_lights {
            objects_guard
                .iter()
                .cloned()
                .chain(self.build_light_objects())
                .collect()
        } else {
            objects_guard.iter().cloned().collect()
        };
        drop(objects_guard);
        self.dispatch_render(renderer, &objects, &self.lights)
    }
}
