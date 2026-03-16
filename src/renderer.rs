use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;

pub struct Renderer;

impl Renderer {
    pub fn draw_object(object: &Object, camera: &Camera, framebuffer: &mut Framebuffer) {
        let model_matrix = object.transform.matrix();
        let view_matrix = camera.view_matrix();
        let projection_matrix = camera.projection_matrix();

        for (i0, i1, i2) in &object.mesh.faces {
            let v0 = object.mesh.vertices[*i0];
            let v1 = object.mesh.vertices[*i1];
            let v2 = object.mesh.vertices[*i2];

            // Transform to world space
            let world_triangle = Triangle::new(v0, v1, v2).transform(model_matrix);

            // Back-face culling in world space
            if world_triangle.is_backface(camera.position) {
                continue;
            }

            // Transform to camera space
            let camera_triangle = world_triangle.transform(view_matrix);

            // Project to screen space — returns 2D points
            let (p0, p1, p2) = camera_triangle.project(
                projection_matrix,
                framebuffer.width as f32,
                framebuffer.height as f32,
            );

            // Build a screen-space triangle for rasterization
            let screen_triangle = Triangle::new(
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
                Vec3::new(p2.x, p2.y, 0.0),
            );

            // Rasterize
            let (min, max) = screen_triangle.bounding_box();
            let min_x = (min.x as i32).max(0);
            let max_x = (max.x as i32).min(framebuffer.width as i32 - 1);
            let min_y = (min.y as i32).max(0);
            let max_y = (max.y as i32).min(framebuffer.height as i32 - 1);

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    if screen_triangle.contains_point(p) {
                        framebuffer.set_pixel(x as usize, y as usize, [255, 0, 0, 255]);
                    }
                }
            }
        }
    }
}
