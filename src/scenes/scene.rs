use crate::{framebuffer::Framebuffer, geometry::object::Object, renderer::Renderer};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

pub struct Scene {
    pub objects: Arc<RwLock<Vec<Object>>>,
    pub framebuffer: Framebuffer,
    pub camera: Camera,
    pub light: Option<PointLight>,
}

impl Scene {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            objects: Arc::new(RwLock::new(Vec::new())),
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            light: None,
        }
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.write().unwrap().push(object);
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
            Renderer::draw_object(object, &self.camera, &self.light, &mut self.framebuffer);
        }
    }
}
