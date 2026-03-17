use crate::{framebuffer::Framebuffer, geometry::object::Object, renderer::Renderer};

use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

pub struct Scene {
    pub objects: Vec<Object>,
    pub framebuffer: Framebuffer,
    pub camera: Camera,
    pub light: Option<PointLight>,
}

impl Scene {
    pub fn new(height: f32, width: f32) -> Self {
        Self {
            objects: Vec::new(),
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            light: None,
        }
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }

    pub fn render_objects(&mut self) {
        for object in &mut self.objects {
            object.transform.rotation.x += 0.01;
            object.transform.rotation.x %= 2.0 * std::f32::consts::PI;
            object.transform.rotation.y += 0.01;
            object.transform.rotation.y %= 2.0 * std::f32::consts::PI;
            Renderer::draw_object(object, &self.camera, &self.light, &mut self.framebuffer);
        }
    }
}
