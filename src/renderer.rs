use crate::framebuffer::Framebuffer;
use crate::geometry::line::Line;
use crate::geometry::object::Object;
use crate::scenes::camera::Camera;

pub struct Renderer;

impl Renderer {
    pub fn draw_object(object: &Object, camera: &Camera, framebuffer: &mut Framebuffer) {
        let mvp = camera.projection_matrix() * camera.view_matrix() * object.transform.matrix();

        for (start, end) in &object.mesh.edges {
            let v0 = object.mesh.vertices[*start];
            let v1 = object.mesh.vertices[*end];

            let clip0 = mvp * v0.to_vec4();
            let clip1 = mvp * v1.to_vec4();

            if clip0.w <= 0.0 || clip1.w <= 0.0 {
                continue;
            }

            let ndc0 = clip0.perspective_divide();
            let ndc1 = clip1.perspective_divide();

            let p0 = ndc0.project_to_2d(framebuffer.width, framebuffer.height);

            let p1 = ndc1.project_to_2d(framebuffer.width, framebuffer.height);

            Line::new(p0, p1).draw(framebuffer);
        }
    }
}
