use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

const AMBIENT: f32 = 0.15;

pub struct Renderer;

impl Renderer {
    pub fn draw_object(
        object: &Object,
        camera: &Camera,
        light_source: &Option<PointLight>,
        framebuffer: &mut Framebuffer,
    ) {
        let model_matrix = object.transform.matrix();
        let view_matrix = camera.view_matrix();
        let projection_matrix = camera.projection_matrix();

        let normal_matrix = model_matrix.inverse().unwrap().transpose();

        for (face_idx, (i0, i1, i2)) in object.mesh.faces.iter().enumerate() {
            let v0 = object.mesh.vertices[*i0];
            let v1 = object.mesh.vertices[*i1];
            let v2 = object.mesh.vertices[*i2];

            let n0 = object.mesh.normals[*i0];
            let n1 = object.mesh.normals[*i1];
            let n2 = object.mesh.normals[*i2];
            let n0_world = (normal_matrix * n0.to_vec4()).to_vec3().normalise();
            let n1_world = (normal_matrix * n1.to_vec4()).to_vec3().normalise();
            let n2_world = (normal_matrix * n2.to_vec4()).to_vec3().normalise();

            // Transform to world space
            let world_triangle = Triangle::new(v0, v1, v2).transform(model_matrix);

            // Back-face culling in world space
            if world_triangle.is_backface(camera.position) {
                continue;
            }

            // Transform to camera space
            let camera_triangle = world_triangle.transform(view_matrix);

            // Project to screen space
            let ((p0, z0), (p1, z1), (p2, z2)) = camera_triangle.project(
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

                    if let Some((w0, w1, w2)) = screen_triangle.contains_point(p) {
                        let depth = w0 * z0 + w1 * z1 + w2 * z2;

                        if framebuffer.test_and_set_depth(x as usize, y as usize, depth) {

                            // Interpolate normal
                            let normal = (n0_world * w0 + n1_world * w1 + n2_world * w2).normalise();

                            let diffuse_intensity = match light_source {
                                Some(light) => {
                                    let centre = world_triangle.centre();
                                    let distance_intensity = light.intensity_at(centre);

                                    let diffuse = light
                                        .direction_to(centre)
                                        .dot(normal)
                                        .max(0.0);

                                    AMBIENT + (1.0 - AMBIENT) * diffuse * distance_intensity
                                }
                                None => AMBIENT,
                            };

                            let base_color = object.mesh.color_of(face_idx);

                            let shaded_color = [
                                (base_color[0] as f32 * diffuse_intensity) as u8,
                                (base_color[1] as f32 * diffuse_intensity) as u8,
                                (base_color[2] as f32 * diffuse_intensity) as u8,
                                base_color[3],
                            ];

                            framebuffer.set_pixel(x as usize, y as usize, shaded_color);
                        }
                    }
                }
            }

        }

        if let Some(light_source) = light_source {
            // For debugging: draw light source as a small white square
            let light_screen_pos = (projection_matrix
                * view_matrix
                * Vec3::new(
                    light_source.position.x,
                    light_source.position.y,
                    light_source.position.z,
                )
                .to_vec4())
            .to_vec3();
            let light_screen_x = ((light_screen_pos.x / light_screen_pos.z + 1.0)
                * 0.5
                * framebuffer.width as f32) as i32;
            let light_screen_y = ((1.0 - light_screen_pos.y / light_screen_pos.z)
                * 0.5
                * framebuffer.height as f32) as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let lx = light_screen_x + dx;
                    let ly = light_screen_y + dy;
                    if lx >= 0
                        && lx < framebuffer.width as i32
                        && ly >= 0
                        && ly < framebuffer.height as i32
                    {
                        framebuffer.set_pixel(lx as usize, ly as usize, [255, 255, 255, 255]);
                    }
                }
            }
        }
    }
}
