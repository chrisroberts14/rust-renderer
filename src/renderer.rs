use crate::framebuffer::Framebuffer;
use crate::geometry::line::Line;
use crate::geometry::object::Object;

pub struct Renderer;

impl Renderer {
    pub fn draw_object(object: &Object, framebuffer: &mut Framebuffer) {
        let model = object.transform.matrix();

        for (start, end) in &object.mesh.edges {
            let v0 = object.mesh.vertices[*start];
            let v1 = object.mesh.vertices[*end];

            let v0_world = model * v0.to_vec4();
            let v1_world = model * v1.to_vec4();

            let p0 = v0_world
                .perspective_divide()
                .project_to_2d(framebuffer.width, framebuffer.height);
            let p1 = v1_world
                .perspective_divide()
                .project_to_2d(framebuffer.width, framebuffer.height);

            let line = Line::new(p0, p1);
            line.draw(framebuffer);
        }
    }
}
