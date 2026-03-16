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

        let width = framebuffer.width as i32;
        let height = framebuffer.height as i32;

        let vertices_world: Vec<Vec3> = object
            .mesh
            .vertices
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
            .map(|n| (normal_matrix * n.to_vec4()).to_vec3())
            .collect();

        for (face_idx, (i0, i1, i2)) in object.mesh.faces.iter().enumerate() {
            let v0 = vertices_cam[*i0];
            let v1 = vertices_cam[*i1];
            let v2 = vertices_cam[*i2];

            let v0_world = vertices_world[*i0];
            let v1_world = vertices_world[*i1];
            let v2_world = vertices_world[*i2];

            let n0 = normals_world[*i0];
            let n1 = normals_world[*i1];
            let n2 = normals_world[*i2];

            // Backface culling
            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let normal = e1.cross(e2);

            if normal.dot(-v0) <= 0.0 {
                continue;
            }

            let ((p0, z0), (p1, z1), (p2, z2)) =
                Triangle::new(v0, v1, v2).project(projection, width as f32, height as f32);

            let x0 = p0.x;
            let y0 = p0.y;
            let x1 = p1.x;
            let y1 = p1.y;
            let x2 = p2.x;
            let y2 = p2.y;

            let min_x = x0.min(x1).min(x2).floor().max(0.0) as i32;
            let max_x = x0.max(x1).max(x2).ceil().min(width as f32 - 1.0) as i32;
            let min_y = y0.min(y1).min(y2).floor().max(0.0) as i32;
            let max_y = y0.max(y1).max(y2).ceil().min(height as f32 - 1.0) as i32;

            // Edge function constants
            let area = (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0);
            if area == 0.0 {
                continue;
            }
            let inv_area = 1.0 / area;

            let lighting = light_source.as_ref().map(|light| {
                let centre = (v0_world + v1_world + v2_world) / 3.0;
                (light.intensity_at(centre), light.direction_to(centre))
            });

            let base_color = object.mesh.color_of(face_idx);
            let r = base_color[0] as f32;
            let g = base_color[1] as f32;
            let b = base_color[2] as f32;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;

                    let w0 = ((x1 - px) * (y2 - py) - (y1 - py) * (x2 - px)) * inv_area;
                    let w1 = ((x2 - px) * (y0 - py) - (y2 - py) * (x0 - px)) * inv_area;
                    let w2 = 1.0 - w0 - w1;

                    if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                        let depth = w0 * z0 + w1 * z1 + w2 * z2;

                        let ux = x as usize;
                        let uy = y as usize;

                        if framebuffer.test_and_set_depth(ux, uy, depth) {
                            let normal = n0 * w0 + n1 * w1 + n2 * w2;

                            let mut diffuse_intensity = AMBIENT;

                            if let Some((distance_intensity, light_dir)) = lighting {
                                let diffuse = light_dir.dot(normal).max(0.0);
                                diffuse_intensity =
                                    AMBIENT + (1.0 - AMBIENT) * diffuse * distance_intensity;
                            }

                            framebuffer.set_pixel(
                                ux,
                                uy,
                                [
                                    (r * diffuse_intensity) as u8,
                                    (g * diffuse_intensity) as u8,
                                    (b * diffuse_intensity) as u8,
                                    base_color[3],
                                ],
                            );
                        }
                    }
                }
            }
        }
    }
}
