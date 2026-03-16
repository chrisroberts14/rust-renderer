use crate::framebuffer::Framebuffer;
use crate::geometry::line::Line;
use crate::geometry::object::Object;
use crate::maths::vec4::Vec4;
use crate::scenes::camera::Camera;

pub struct Renderer;

impl Renderer {
    pub fn draw_object(object: &Object, camera: &Camera, framebuffer: &mut Framebuffer) {
        let model_matrix = object.transform.matrix();
        let view_matrix = camera.view_matrix();
        let projection_matrix = camera.projection_matrix();

        for (start, end) in &object.mesh.edges {
            // Transform vertices to world space
            let v0_world = model_matrix * object.mesh.vertices[*start].to_vec4();
            let v1_world = model_matrix * object.mesh.vertices[*end].to_vec4();

            // Transform to camera space
            let v0_cam = view_matrix * v0_world;
            let v1_cam = view_matrix * v1_world;

            // Transform to clip space
            let mut clip0 = projection_matrix * v0_cam;
            let mut clip0_test = projection_matrix * v0_cam;
            let mut clip1 = projection_matrix * v1_cam;
            let mut clip1_test = projection_matrix * v1_cam;

            // --- FULL 6-PLANE CLIPPING ---
            if !Self::clip_line(&mut clip0, &mut clip1) {
                continue; // fully outside frustum
            }

            // --- PERSPECTIVE DIVIDE ---
            let ndc0 = match clip0.perspective_divide() {
                Ok(ndc) => ndc,
                Err(_) => continue,
            };
            let ndc1 = match clip1.perspective_divide() {
                Ok(ndc) => ndc,
                Err(_) => continue,
            };

             let ndc0_orig = match clip0_test.perspective_divide() {
                Ok(ndc) => ndc,
                Err(_) => continue,
            };
            let ndc1_orig = match clip1_test.perspective_divide() {
                Ok(ndc) => ndc,
                Err(_) => continue,
            };

            // --- NDC TO SCREEN ---
            let p0_clipped = ndc0.project_to_2d(framebuffer.width, framebuffer.height);
            let p1_clipped = ndc1.project_to_2d(framebuffer.width, framebuffer.height);

            let p0_orig = ndc0_orig.project_to_2d(framebuffer.width, framebuffer.height);
            let p1_orig = ndc1_orig.project_to_2d(framebuffer.width, framebuffer.height);

            // Draw the line
            //Line::new(p0, p1).draw(framebuffer, [0, 255, 0, 255]);
            // Before clipping: white
            Line::new(p0_orig, p1_orig).draw(framebuffer, [255, 0, 0, 255]);
            // After clipping: red
            Line::new(p0_clipped, p1_clipped).draw(framebuffer, [0, 255, 0, 255]);
        }
    }

    /// Clip a line segment in clip space against the 6 frustum planes
    /// Returns false if the line is completely outside
    fn clip_line(v0: &mut Vec4, v1: &mut Vec4) -> bool {
        // Planes: left, right, bottom, top, near, far
        let planes = [
            |v: &Vec4| v.x + v.w,   // left
            |v: &Vec4| v.w - v.x,   // right
            |v: &Vec4| v.y + v.w,   // bottom
            |v: &Vec4| v.w - v.y,   // top
            |v: &Vec4| v.z,         // near (z >= 0)
            |v: &Vec4| v.w - v.z,   // far (z <= w)
        ];

        for plane in planes {
            let d0 = plane(v0);
            let d1 = plane(v1);

            // Completely outside
            if d0 < 0.0 && d1 < 0.0 {
                return false;
            }

            // Line crosses plane
            if d0 * d1 < 0.0 {
                let t = d0 / (d0 - d1);
                let new_point = *v0 + (*v1 - *v0) * t;
                if d0 < 0.0 {
                    *v0 = new_point;
                } else {
                    *v1 = new_point;
                }
            }
        }

        true
    }
}
