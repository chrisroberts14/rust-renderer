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
        let model = object.transform.matrix();
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let model_view = view * model;

        let normal_matrix = model.inverse().unwrap().transpose();

        let vertices_world: Vec<Vec3> = object.mesh.vertices
            .iter()
            .map(|v| (model * v.to_vec4()).to_vec3())
            .collect();

        let vertices_cam: Vec<Vec3> = object
            .mesh
            .vertices
            .iter()
            .map(|v| (model_view * v.to_vec4()).to_vec3())
            .collect();

        let normals_world: Vec<Vec3> = object
            .mesh
            .normals
            .iter()
            .map(|n| (normal_matrix * n.to_vec4()).to_vec3().normalise())
            .collect();

        for (face_idx, (i0, i1, i2)) in object.mesh.faces.iter().enumerate() {
            let v0 = vertices_cam[*i0];
            let v1 = vertices_cam[*i1];
            let v2 = vertices_cam[*i2];

            let v0_world = vertices_world[*i0];
            let v1_world = vertices_world[*i1];
            let v2_world = vertices_world[*i2];

            let n0_world = normals_world[*i0];
            let n1_world = normals_world[*i1];
            let n2_world = normals_world[*i2];

            let e1 = v1 - v0;
            let e2 = v2 - v0;

            let normal = e1.cross(e2);

            if normal.dot(-v0) <= 0.0 {
                continue;
            }

            // Project to screen space
            let ((p0, z0), (p1, z1), (p2, z2)) = Triangle::new(v0, v1, v2).project(
                projection,
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

            let lighting = match light_source {
                Some(light) => {
                    let centre = (v0_world + v1_world + v2_world) / 3.0;
                    let distance_intensity = light.intensity_at(centre);
                    let light_dir = light.direction_to(centre);

                    Some((distance_intensity, light_dir))
                }
                None => None,
            };

            let base_color = object.mesh.color_of(face_idx);
            let r = base_color[0] as f32;
            let g = base_color[1] as f32;
            let b = base_color[2] as f32;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    if let Some((w0, w1, w2)) = screen_triangle.contains_point(x as f32 + 0.5, y as f32 +0.5) {
                        let depth = w0 * z0 + w1 * z1 + w2 * z2;

                        let ux = x as usize;
                        let uy = y as usize;

                        if framebuffer.test_and_set_depth(ux, uy, depth) {

                            // Interpolate normal
                            //let normal = (n0_world * w0 + n1_world * w1 + n2_world * w2).normalise();

                            let normal = n0_world * w0 + n1_world * w1 + n2_world * w2;
                            //let diffuse = light_dir.dot(normal).max(0.0);
                            
                            let diffuse_intensity = match lighting {
                                Some((distance_intensity, light_dir)) => {
                                    let diffuse = light_dir.dot(normal).max(0.0);
                                    AMBIENT + (1.0 - AMBIENT) * diffuse * distance_intensity
                                }
                                None => AMBIENT,
                            };

                            let shaded_color = [
                                (r * diffuse_intensity) as u8,
                                (g * diffuse_intensity) as u8,
                                (b * diffuse_intensity) as u8,
                                base_color[3],
                            ];

                            framebuffer.set_pixel(ux, uy, shaded_color);
                        }
                    }
                }
            }

        }

        /*if let Some(light_source) = light_source {
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
        }*/
    }
}
