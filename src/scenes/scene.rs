use crate::geometry::cube::Cube;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;
use crate::{framebuffer::Framebuffer, geometry::object::Object, renderer::Renderer};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

pub struct Scene {
    pub objects: Arc<RwLock<Vec<Object>>>,
    pub framebuffer: Framebuffer,
    pub camera: Camera,
    pub lights: Vec<PointLight>,
}

impl Scene {
    pub fn new(width: f32, height: f32, objects: Vec<Object>, lights: Vec<PointLight>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(objects)),
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            lights,
        }
    }

    /// Spawn a thread that continuously updates object transforms
    pub fn spawn_update_thread(&self) -> thread::JoinHandle<()> {
        let objects = Arc::clone(&self.objects);
        thread::spawn(move || {
            loop {
                {
                    let mut objs = objects.write().unwrap();
                    for object in objs.iter_mut() {
                        object.transform.rotation.x += 0.01;
                        object.transform.rotation.x %= 2.0 * std::f32::consts::PI;
                        object.transform.rotation.y += 0.01;
                        object.transform.rotation.y %= 2.0 * std::f32::consts::PI;
                    }
                } // write lock dropped here
                thread::sleep(Duration::from_millis(16));
            }
        })
    }

    pub fn render_objects(&mut self) {
        let objects = self.objects.read().unwrap();
        for object in objects.iter() {
            Renderer::draw_object(object, &self.camera, &self.lights, &mut self.framebuffer);
        }
    }

    /// Render small box representations around the point lights for debugging purposes
    /// In order to actually see it we need it to be lit without needing another light
    pub fn render_lights(&mut self) {
        for light in self.lights.iter() {
            // Convert the lights colour from [0.0, 1.0] to [0, 255] for the framebuffer
            let colour = [
                (light.colour[0] * 255.0) as u8,
                (light.colour[1] * 255.0) as u8,
                (light.colour[2] * 255.0) as u8,
                255,
            ];

            let light_box = Object::new(
                Cube::mesh(1.0, colour),
                Transform {
                    position: light.position,
                    rotation: Vec3::new(0.0, 0.0, 0.0),
                    scale: Vec3::new(0.1, 0.1, 0.1),
                },
            );
            Renderer::draw_object(&light_box, &self.camera, &[], &mut self.framebuffer);
        }
    }
}
