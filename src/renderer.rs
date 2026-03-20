use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

const AMBIENT: f32 = 0.15;
const SHININESS: i32 = 32;
pub struct Renderer;

// A vertex bundle: (camera-space position, world-space position, world-space normal, texture UV)
#[derive(Clone, Copy)]
struct Vert {
    cam: Vec3,
    world: Vec3,
    normal: Vec3,
    uv: Vec2,
}

fn interpolate_vert(a: Vert, b: Vert, t: f32) -> Vert {
    Vert {
        cam: a.cam * (1.0 - t) + b.cam * t,
        world: a.world * (1.0 - t) + b.world * t,
        normal: a.normal * (1.0 - t) + b.normal * t,
        uv: a.uv * (1.0 - t) + b.uv * t,
    }
}

/// Clips a triangle against the near plane (z = -near in camera space).
/// Returns 0, 1, or 2 triangles.
fn clip_near(vertices: [Vert; 3], near: f32) -> Vec<[Vert; 3]> {
    let inside: [bool; 3] = vertices.map(|v| v.cam.z <= -near);
    let n_inside = inside.iter().filter(|&&b| b).count();

    match n_inside {
        0 => vec![],
        3 => vec![vertices],
        1 => {
            let in_idx = (0..3).find(|&i| inside[i]).unwrap();
            let [out0, out1] = (0..3)
                .filter(|&i| !inside[i])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let a: Vert = vertices[in_idx];
            let b: Vert = vertices[out0];
            let c: Vert = vertices[out1];

            let ab = interpolate_vert(a, b, (-near - a.cam.z) / (b.cam.z - a.cam.z));
            let ac = interpolate_vert(a, c, (-near - a.cam.z) / (c.cam.z - a.cam.z));

            vec![[a, ab, ac]]
        }
        2 => {
            let out_idx = (0..3).find(|&i| !inside[i]).unwrap();
            let [in0, in1] = (0..3)
                .filter(|&i| inside[i])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let a: Vert = vertices[in0];
            let b: Vert = vertices[in1];
            let c: Vert = vertices[out_idx];

            let ac = interpolate_vert(a, c, (-near - a.cam.z) / (c.cam.z - a.cam.z));
            let bc = interpolate_vert(b, c, (-near - b.cam.z) / (c.cam.z - b.cam.z));

            vec![[a, b, bc], [a, bc, ac]]
        }
        _ => unreachable!(),
    }
}

/// Computes the Phong light multiplier [r, g, b] for a surface point.
/// Returns [1.0; 3] when there are no lights (unlit rendering).
fn shade(normal: Vec3, world_pos: Vec3, view_dir: Vec3, lights: &[PointLight]) -> [f32; 3] {
    if lights.is_empty() {
        return [1.0; 3];
    }
    let mut diffuse_rgb = [0.0f32; 3];
    let mut specular_rgb = [0.0f32; 3];
    for light in lights.iter() {
        let light_colour = light.colour_at(world_pos);
        let light_dir = light.direction_to(world_pos);
        let diffuse = light_dir.dot(normal).max(0.0);
        let reflect = normal * (2.0 * normal.dot(light_dir)) - light_dir;
        let specular = reflect.dot(view_dir).max(0.0).powi(SHININESS);
        for i in 0..3 {
            diffuse_rgb[i] += diffuse * light_colour[i];
            specular_rgb[i] += specular * light_colour[i];
        }
    }
    std::array::from_fn(|i| (AMBIENT + (1.0 - AMBIENT) * diffuse_rgb[i] + specular_rgb[i]).min(1.0))
}

impl Renderer {
    pub fn draw_object(
        object: &Object,
        camera: &Camera,
        lights: &[PointLight],
        framebuffer: &mut Framebuffer,
        wire_frame_mode: bool,
    ) {
        let (model, normal_matrix) = object.transform.matrices();
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let model_view = view * model;

        let width = framebuffer.width as i32;
        let height = framebuffer.height as i32;

        let verts: Vec<Vert> = object
            .mesh
            .vertices
            .iter()
            .zip(object.mesh.normals.iter())
            .map(|(v, n)| Vert {
                cam: (model_view * v.to_vec4()).to_vec3(),
                world: (model * v.to_vec4()).to_vec3(),
                normal: (normal_matrix * n.to_vec4()).to_vec3(),
                uv: Vec2::new(0.0, 0.0),
            })
            .collect();

        for (face_idx, (i0, i1, i2)) in object.mesh.faces.iter().enumerate() {
            // Build per-face verts with correct UVs (UV indices differ from vertex indices)
            let (uv_i0, uv_i1, uv_i2) = object
                .mesh
                .uv_faces
                .get(face_idx)
                .copied()
                .unwrap_or((0, 0, 0));
            let zero_uv = Vec2::new(0.0, 0.0);
            let mut v0 = verts[*i0];
            let mut v1 = verts[*i1];
            let mut v2 = verts[*i2];
            v0.uv = *object.mesh.uvs.get(uv_i0).unwrap_or(&zero_uv);
            v1.uv = *object.mesh.uvs.get(uv_i1).unwrap_or(&zero_uv);
            v2.uv = *object.mesh.uvs.get(uv_i2).unwrap_or(&zero_uv);

            let clipped = clip_near([v0, v1, v2], camera.near);
            if clipped.is_empty() {
                continue;
            }

            let face_color = object.mesh.color_of(face_idx);

            for [v0, v1, v2] in clipped {
                let ((p0, z0), (p1, z1), (p2, z2)) = Triangle::new(v0.cam, v1.cam, v2.cam).project(
                    projection,
                    width as f32,
                    height as f32,
                );

                let screen_tri = Triangle::new(
                    Vec3::new(p0.x, p0.y, 0.0),
                    Vec3::new(p1.x, p1.y, 0.0),
                    Vec3::new(p2.x, p2.y, 0.0),
                );

                if wire_frame_mode {
                    framebuffer.draw_triangle_wireframe(&screen_tri);
                    continue;
                }

                let (min, max) = screen_tri.bounding_box();
                let min_x = (min.x.floor() as i32).max(0);
                let max_x = (max.x.ceil() as i32).min(width - 1);
                let min_y = (min.y.floor() as i32).max(0);
                let max_y = (max.y.ceil() as i32).min(height - 1);

                let area = (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y);

                if area <= 0.0 {
                    continue;
                }
                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;

                        if let Some((w0, w1, w2)) = screen_tri.contains_point(px, py) {
                            let depth = w0 * z0 + w1 * z1 + w2 * z2;
                            let ux = x as usize;
                            let uy = y as usize;

                            if framebuffer.test_and_set_depth(ux, uy, depth) {
                                let normal =
                                    (v0.normal * w0 + v1.normal * w1 + v2.normal * w2).normalise();
                                let world_pos = v0.world * w0 + v1.world * w1 + v2.world * w2;
                                let view_dir = (camera.position - world_pos).normalise();

                                let [lr, lg, lb] = shade(normal, world_pos, view_dir, lights);

                                // Perspective-correct UV interpolation (use camera-space z, negate since cam.z is negative)
                                let inv_z0 = -1.0 / v0.cam.z;
                                let inv_z1 = -1.0 / v1.cam.z;
                                let inv_z2 = -1.0 / v2.cam.z;
                                let inv_z = w0 * inv_z0 + w1 * inv_z1 + w2 * inv_z2;
                                let uv = (v0.uv * inv_z0 * w0
                                    + v1.uv * inv_z1 * w1
                                    + v2.uv * inv_z2 * w2)
                                    / inv_z;

                                let [cr, cg, cb, ca] = if let Some(tex) = &object.texture {
                                    tex.sample(uv.x, uv.y)
                                } else {
                                    face_color
                                };
                                let (r, g, b) = (cr as f32, cg as f32, cb as f32);

                                framebuffer.set_pixel(
                                    ux,
                                    uy,
                                    [(r * lr) as u8, (g * lg) as u8, (b * lb) as u8, ca],
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
