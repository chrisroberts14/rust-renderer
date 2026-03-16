use crate::{framebuffer::Framebuffer, geometry::object::Object, renderer::Renderer};

use crate::scenes::camera::Camera;

pub struct Scene {
    pub objects: Vec<Object>,
    pub camera: Camera,
}

impl Scene {
    pub fn new(height: f32, width: f32) -> Self {
        Self {
            objects: Vec::new(),
            camera: Camera::new(height, width),
        }
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }

    pub fn render_objects(&mut self, framebuffer: &mut Framebuffer) {
        for object in &mut self.objects {
            object.transform.rotation.x += 0.01;
            object.transform.rotation.y += 0.01;
            Renderer::draw_object(object, &self.camera, framebuffer);
        }
    }
}
