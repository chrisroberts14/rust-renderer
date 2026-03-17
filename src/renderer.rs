use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

const AMBIENT: f32 = 0.15;

pub struct Renderer;

// A vertex bundle: (camera-space position, world-space position, world-space normal)
type Vert = (Vec3, Vec3, Vec3);

fn interpolate_vert(a: Vert, b: Vert, t: f32) -> Vert {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// Clips a triangle against the near plane (z = -near in camera space).
/// Returns 0, 1, or 2 triangles.
fn clip_near(vertices: [Vert; 3], near: f32) -> Vec<[Vert; 3]> {
    let inside: [bool; 3] = vertices.map(|v| v.0.z <= -near);

    // How many vertices of the triangle are inside the near plane?
    let n_inside = inside.iter().filter(|&&b| b).count();

    match n_inside {
        0 => vec![],
        3 => vec![vertices],
        1 => {
            // Find the single vertex that's inside and the two that are outside, then compute the intersection points.
            let in_idx = (0..3).find(|&i| inside[i]).unwrap();
            let out_idx: Vec<usize> = (0..3).filter(|&i| !inside[i]).collect();
            let a = vertices[in_idx];
            let b = vertices[out_idx[0]];
            let c = vertices[out_idx[1]];
            let t_ab = (-near - a.0.z) / (b.0.z - a.0.z);
            let t_ac = (-near - a.0.z) / (c.0.z - a.0.z);
            vec![[
                a,
                interpolate_vert(a, b, t_ab),
                interpolate_vert(a, c, t_ac),
            ]]
        }
        2 => {
            // Find the single vertex that's outside and the two that are inside, then compute the intersection points.
            let out_idx = (0..3).find(|&i| !inside[i]).unwrap();
            let in_idx: Vec<usize> = (0..3).filter(|&i| inside[i]).collect();
            let a = vertices[in_idx[0]];
            let b = vertices[in_idx[1]];
            let c = vertices[out_idx];
            let t_ac = (-near - a.0.z) / (c.0.z - a.0.z);
            let t_bc = (-near - b.0.z) / (c.0.z - b.0.z);
            let ac = interpolate_vert(a, c, t_ac);
            let bc = interpolate_vert(b, c, t_bc);
            vec![[a, b, bc], [a, bc, ac]]
        }
        _ => unreachable!(),
    }
}

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

            let cam_tri = Triangle::new(v0, v1, v2);

            // Backface culling in camera space (camera is at origin)
            if cam_tri.is_backface(Vec3::new(0.0, 0.0, 0.0)) {
                continue;
            }

            let verts: [Vert; 3] = [
                (v0, vertices_world[*i0], normals_world[*i0]),
                (v1, vertices_world[*i1], normals_world[*i1]),
                (v2, vertices_world[*i2], normals_world[*i2]),
            ];

            let clipped = clip_near(verts, camera.near);
            if clipped.is_empty() {
                continue;
            }

            let base_color = object.mesh.color_of(face_idx);
            let r = base_color[0] as f32;
            let g = base_color[1] as f32;
            let b = base_color[2] as f32;

            for [(v0_c, v0_w, n0), (v1_c, v1_w, n1), (v2_c, v2_w, n2)] in clipped {
                let ((p0, z0), (p1, z1), (p2, z2)) = Triangle::new(v0_c, v1_c, v2_c).project(
                    projection,
                    width as f32,
                    height as f32,
                );

                let screen_tri = Triangle::new(
                    Vec3::new(p0.x, p0.y, 0.0),
                    Vec3::new(p1.x, p1.y, 0.0),
                    Vec3::new(p2.x, p2.y, 0.0),
                );

                let (min, max) = screen_tri.bounding_box();
                let min_x = (min.x.floor() as i32).max(0);
                let max_x = (max.x.ceil() as i32).min(width - 1);
                let min_y = (min.y.floor() as i32).max(0);
                let max_y = (max.y.ceil() as i32).min(height - 1);

                let lighting = light_source.as_ref().map(|light| {
                    let centre = (v0_w + v1_w + v2_w) / 3.0;
                    (light.intensity_at(centre), light.direction_to(centre))
                });

                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let px = x as f32 + 0.5;
                        let py = y as f32 + 0.5;

                        if let Some((w0, w1, w2)) = screen_tri.contains_point(px, py) {
                            let depth = w0 * z0 + w1 * z1 + w2 * z2;
                            let ux = x as usize;
                            let uy = y as usize;

                            if framebuffer.test_and_set_depth(ux, uy, depth) {
                                let normal = n0 * w0 + n1 * w1 + n2 * w2;

                                let diffuse_intensity =
                                    if let Some((distance_intensity, light_dir)) = lighting {
                                        let diffuse = light_dir.dot(normal).max(0.0);
                                        AMBIENT + (1.0 - AMBIENT) * diffuse * distance_intensity
                                    } else {
                                        AMBIENT
                                    };

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
}
