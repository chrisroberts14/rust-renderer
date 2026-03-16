use crate::{framebuffer::Framebuffer, geometry::object::Object, renderer::Renderer};

pub struct SceneObjects {
    pub objects: Vec<Object>,
}

impl SceneObjects {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }

    pub fn render_objects(&mut self, framebuffer: &mut Framebuffer) {
        for object in &mut self.objects {
            object.transform.rotation.x += 0.01;
            object.transform.rotation.y += 0.01;
            Renderer::draw_object(object, framebuffer);
        }
    }
}
