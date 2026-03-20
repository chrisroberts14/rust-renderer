use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::material::Material;
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
        // Compute the model matrix and its inverse-transpose (for correct normal transformation
        // under non-uniform scaling), plus the view and projection matrices.
        let (model, normal_matrix) = object.transform.matrices();
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let model_view = view * model;

        let width = framebuffer.width as i32;
        let height = framebuffer.height as i32;

        // Transform every vertex into camera space and world space once up front.
        // UVs are left as zero here because UV indices can differ from vertex indices
        // in OBJ files — they are patched per-face below.
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

        let get_uv = |i: usize| {
            object
                .mesh
                .uvs
                .get(i)
                .copied()
                .unwrap_or(Vec2::new(0.0, 0.0))
        };

        for (face_idx, (i0, i1, i2)) in object.mesh.faces.iter().enumerate() {
            // Build per-face verts with correct UVs (UV indices differ from vertex indices)
            let (uv_i0, uv_i1, uv_i2) = object
                .mesh
                .uv_faces
                .get(face_idx)
                .copied()
                .unwrap_or((0, 0, 0));
            let v0 = Vert {
                uv: get_uv(uv_i0),
                ..verts[*i0]
            };
            let v1 = Vert {
                uv: get_uv(uv_i1),
                ..verts[*i1]
            };
            let v2 = Vert {
                uv: get_uv(uv_i2),
                ..verts[*i2]
            };

            // Clip against the near plane. This may produce 0, 1, or 2 triangles.
            let clipped = clip_near([v0, v1, v2], camera.near);
            if clipped.is_empty() {
                continue;
            }

            for [v0, v1, v2] in clipped {
                // Project camera-space positions to 2D screen coordinates.
                // z values are NDC depth, kept for interpolation later.
                let ((p0, z0), (p1, z1), (p2, z2)) = Triangle::new(v0.cam, v1.cam, v2.cam).project(
                    projection,
                    width as f32,
                    height as f32,
                );

                // Screen-space triangle used for bounding box and point containment tests.
                let screen_tri = Triangle::new(
                    Vec3::new(p0.x, p0.y, 0.0),
                    Vec3::new(p1.x, p1.y, 0.0),
                    Vec3::new(p2.x, p2.y, 0.0),
                );

                if wire_frame_mode {
                    framebuffer.draw_triangle_wireframe(&screen_tri);
                    continue;
                }

                // Clamp the rasterization bounds to the screen.
                let (min, max) = screen_tri.bounding_box();
                let min_x = (min.x.floor() as i32).max(0);
                let max_x = (max.x.ceil() as i32).min(width - 1);
                let min_y = (min.y.floor() as i32).max(0);
                let max_y = (max.y.ceil() as i32).min(height - 1);

                // Signed area of the screen-space triangle. Negative means back-facing
                // (winding reversed after projection) so we skip it — backface culling.
                let area = (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y);
                if area <= 0.0 {
                    continue;
                }

                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        // Test pixel centre against the triangle.
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;

                        if let Some((w0, w1, w2)) = screen_tri.contains_point(px, py) {
                            // Interpolate depth and run the depth test before doing any shading work.
                            let depth = w0 * z0 + w1 * z1 + w2 * z2;
                            let ux = x as usize;
                            let uy = y as usize;

                            if framebuffer.test_and_set_depth(ux, uy, depth) {
                                // Interpolate vertex attributes across the triangle.
                                let normal =
                                    (v0.normal * w0 + v1.normal * w1 + v2.normal * w2).normalise();
                                let world_pos = v0.world * w0 + v1.world * w1 + v2.world * w2;
                                let view_dir = (camera.position - world_pos).normalise();

                                let [lr, lg, lb] = shade(normal, world_pos, view_dir, lights);

                                let [cr, cg, cb, ca] = match &object.material {
                                    Material::Color(c) => *c,
                                    // Perspective-correct UV interpolation: divide UV by camera-space z
                                    // (negated since cam.z is negative for visible geometry) before
                                    // interpolating, then divide out the 1/z factor afterwards.
                                    Material::Texture(tex) => {
                                        let inv_z0 = -1.0 / v0.cam.z;
                                        let inv_z1 = -1.0 / v1.cam.z;
                                        let inv_z2 = -1.0 / v2.cam.z;
                                        let inv_z = w0 * inv_z0 + w1 * inv_z1 + w2 * inv_z2;
                                        let uv = (v0.uv * inv_z0 * w0
                                            + v1.uv * inv_z1 * w1
                                            + v2.uv * inv_z2 * w2)
                                            / inv_z;
                                        tex.sample(uv.x, uv.y)
                                    }
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
