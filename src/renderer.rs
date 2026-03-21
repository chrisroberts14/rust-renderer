use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::material::Material;
use crate::scenes::pointlight::PointLight;
use crate::tile::Tile;

const AMBIENT: f32 = 0.15;
const SHININESS: i32 = 32;
pub(crate) const TILE_SIZE: usize = 32;
pub struct Renderer;

// A vertex bundle: (camera-space position, world-space position, world-space normal, texture UV)
#[derive(Clone, Copy)]
struct Vert {
    cam: Vec3,
    world: Vec3,
    normal: Vec3,
    uv: Vec2,
}

/// A triangle with everything needed to rasterize
pub struct PreparedTriangle {
    verts: [Vert; 3],
    screen: [Vec2; 3],
    depths: [f32; 3],
    material: Material,
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

        // Calculate the normal and light dot product once
        let ndotl = normal.dot(light_dir).max(0.0);
        let diffuse = ndotl;

        // If the light is behind the surface specular is guaranteed to be 0
        let mut specular = 0.0;
        if ndotl > 0.0 {
            let reflect = normal * (2.0 * ndotl) - light_dir;
            specular = reflect.dot(view_dir).max(0.0).powi(SHININESS);
        }

        for i in 0..3 {
            diffuse_rgb[i] += diffuse * light_colour[i];
            specular_rgb[i] += specular * light_colour[i];
        }
    }
    let inv_ambient = 1.0 - AMBIENT;
    [
        (AMBIENT + inv_ambient * diffuse_rgb[0] + specular_rgb[0]).min(1.0),
        (AMBIENT + inv_ambient * diffuse_rgb[1] + specular_rgb[1]).min(1.0),
        (AMBIENT + inv_ambient * diffuse_rgb[2] + specular_rgb[2]).min(1.0),
    ]
}

/// Binning pass: assigns each triangle to every tile whose bounds overlap its screen bounding box.
/// Returns one `Vec<usize>` per tile, containing indices into `triangles`.
pub(crate) fn bin_triangles(triangles: &[PreparedTriangle], tiles: &[Tile]) -> Vec<Vec<usize>> {
    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); tiles.len()];

    for (tri_idx, tri) in triangles.iter().enumerate() {
        let [p0, p1, p2] = tri.screen;

        // Screen-space bounding box of this triangle.
        let min_x = p0.x.min(p1.x).min(p2.x);
        let max_x = p0.x.max(p1.x).max(p2.x);
        let min_y = p0.y.min(p1.y).min(p2.y);
        let max_y = p0.y.max(p1.y).max(p2.y);

        for (tile_idx, tile) in tiles.iter().enumerate() {
            let tx = tile.x as f32;
            let ty = tile.y as f32;
            let tx_end = (tile.x + tile.width) as f32;
            let ty_end = (tile.y + tile.height) as f32;

            // AABB overlap: the triangle touches this tile if its bounding box intersects the tile rect.
            if max_x >= tx && min_x < tx_end && max_y >= ty && min_y < ty_end {
                bins[tile_idx].push(tri_idx);
            }
        }
    }

    bins
}

impl Renderer {
    /// Geometry pass: transforms, clips, projects, and backface-culls all faces of an object.
    /// Returns a flat list of screen-ready triangles with no framebuffer writes.
    pub fn prepare_object(
        object: &Object,
        width: f32,
        height: f32,
        camera_view_mat: Mat4,
        camera_projection_mat: Mat4,
        camera_near: f32,
    ) -> Vec<PreparedTriangle> {
        // Compute the model matrix and its inverse-transpose (for correct normal transformation
        // under non-uniform scaling)
        let (model, normal_matrix) = object.transform.matrices();
        let model_view = camera_view_mat * model;

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

        let mut triangles = Vec::new();

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
            for [v0, v1, v2] in clip_near([v0, v1, v2], camera_near) {
                // Project camera-space positions to 2D screen coordinates.
                // z values are NDC depth, kept for depth interpolation during rasterization.
                let ((p0, z0), (p1, z1), (p2, z2)) = Triangle::new(v0.cam, v1.cam, v2.cam).project(
                    camera_projection_mat,
                    width,
                    height,
                );

                // Signed area of the screen-space triangle. Negative means back-facing
                // (winding reversed after projection) so we skip it — backface culling.
                let area = (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y);
                if area <= 0.0 {
                    continue;
                }

                triangles.push(PreparedTriangle {
                    verts: [v0, v1, v2],
                    screen: [p0, p1, p2],
                    depths: [z0, z1, z2],
                    material: object.material.clone(),
                });
            }
        }

        triangles
    }

    /// Rasterizes all triangles assigned to a tile, clamping pixel iteration to the tile bounds.
    pub(crate) fn rasterize_tile(
        tile: &Tile,
        triangle_indices: &[usize],
        triangles: &[PreparedTriangle],
        camera: &Camera,
        lights: &[PointLight],
        framebuffer: &Framebuffer,
    ) {
        let tile_min_x = tile.x as i32;
        let tile_min_y = tile.y as i32;
        let tile_max_x = (tile.x + tile.width) as i32 - 1;
        let tile_max_y = (tile.y + tile.height) as i32 - 1;

        for &tri_idx in triangle_indices {
            let tri = &triangles[tri_idx];
            let [p0, p1, p2] = tri.screen;
            let [z0, z1, z2] = tri.depths;
            let [v0, v1, v2] = tri.verts;

            // Screen-space triangle used for bounding box and point containment tests.
            let screen_tri = Triangle::new(
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
                Vec3::new(p2.x, p2.y, 0.0),
            );

            // Clamp rasterization bounds to the tile (already backface-culled in prepare_object).
            let (min, max) = screen_tri.bounding_box();
            let min_x = (min.x.floor() as i32).max(tile_min_x);
            let max_x = (max.x.ceil() as i32).min(tile_max_x);
            let min_y = (min.y.floor() as i32).max(tile_min_y);
            let max_y = (max.y.ceil() as i32).min(tile_max_y);

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

                            let [cr, cg, cb, ca] = match &tri.material {
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

    /// Draws all triangles as wireframes, used when wireframe mode is enabled.
    pub(crate) fn draw_wireframe(triangles: &[PreparedTriangle], framebuffer: &Framebuffer) {
        for tri in triangles {
            let [p0, p1, p2] = tri.screen;
            let screen_tri = Triangle::new(
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
                Vec3::new(p2.x, p2.y, 0.0),
            );
            framebuffer.draw_triangle_wireframe(&screen_tri);
        }
    }
}
