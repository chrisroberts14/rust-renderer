use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::lights::Light;

const SHADOW_BIAS: f32 = 0.005;

/// A depth-only render target capturing scene geometry from a light's point of view.
/// Built once per frame per light, then sampled during shading to determine occlusion.
pub struct ShadowMap {
    depth: Vec<f32>,
    width: usize,
    height: usize,
    /// The combined view-projection matrix used to build this map, reused for sampling.
    light_view_proj: Mat4,
}

impl ShadowMap {
    fn new(width: usize, height: usize, light_view_proj: Mat4) -> Self {
        Self {
            depth: vec![f32::MAX; width * height],
            width,
            height,
            light_view_proj,
        }
    }

    fn write_depth(&mut self, x: usize, y: usize, depth: f32) {
        let idx = y * self.width + x;
        if depth < self.depth[idx] {
            self.depth[idx] = depth;
        }
    }

    /// Returns `1.0` (fully lit) or `0.0` (fully in shadow) for a world-space position.
    ///
    /// Points outside the light's frustum are treated as unoccluded.
    pub fn shadow_factor(&self, world_pos: Vec3) -> f32 {
        let clip = self.light_view_proj * world_pos.to_vec4();
        if clip.w <= 0.0 {
            return 1.0;
        }
        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let ndc_z = clip.z / clip.w;

        // Outside the light's frustum — no occlusion data, treat as lit.
        let unit = -1.0..=1.0;
        if !unit.contains(&ndc_x) || !unit.contains(&ndc_y) || !unit.contains(&ndc_z) {
            return 1.0;
        }

        let u = (ndc_x + 1.0) * 0.5;
        let v = (1.0 - ndc_y) * 0.5;

        let sx = ((u * self.width as f32) as usize).min(self.width - 1);
        let sy = ((v * self.height as f32) as usize).min(self.height - 1);

        let stored = self.depth[sy * self.width + sx];

        // stored == f32::MAX means no geometry wrote to this texel → lit.
        // ndc_z > stored + bias means this point is behind the closest surface → shadow.
        if ndc_z > stored + SHADOW_BIAS {
            0.0
        } else {
            1.0
        }
    }
}

/// Builds a view matrix and FOV for a light's shadow pass, shared by both the CPU and GPU renderers.
///
/// Uses the same view-matrix layout as [`Camera::view_matrix`] so that NDC conventions
/// (Y up) are consistent between the shadow pass and the sample lookup.
/// Returns `(view_matrix, fov_radians)`.
pub(super) fn light_view_and_fov(light: &dyn Light) -> (Mat4, f32) {
    let pos = light.position();

    let forward = if let Some(dir) = light.spot_direction() {
        dir.normalise()
    } else {
        // Point light: aim toward the scene origin as a reasonable default.
        let to_origin = Vec3::ZERO - pos;
        if to_origin.length() < 0.001 {
            Vec3::new(0.0, -1.0, 0.0)
        } else {
            to_origin.normalise()
        }
    };

    // Choose an up hint that is never parallel to forward.
    let up_hint = if forward.dot(Vec3::new(0.0, 1.0, 0.0)).abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    // Mirror Camera::view_matrix: r = up_hint × forward, u = forward × right.
    let r = up_hint.cross(forward).normalise();
    let u = forward.cross(r).normalise();
    let view = Mat4 {
        m: [
            [r.x, r.y, r.z, -r.dot(pos)],
            [u.x, u.y, u.z, -u.dot(pos)],
            [-forward.x, -forward.y, -forward.z, forward.dot(pos)],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    // Spot lights use a FOV that covers their full cone + falloff band.
    // Point lights fall back to a 90° perspective frustum (covers one hemisphere).
    let fov = if light.spot_direction().is_some() {
        ((light.cone_angle() + light.falloff_angle()) * 2.0).max(0.1)
    } else {
        std::f32::consts::FRAC_PI_2
    };

    (view, fov)
}

fn light_view_proj(light: &dyn Light, near: f32, far: f32) -> Mat4 {
    let (view, fov) = light_view_and_fov(light);
    let proj = Mat4::perspective(fov, 1.0, near, far);
    proj * view
}

/// Rasterizes all non-light objects into a depth-only shadow map from `light`'s point of view.
pub fn build_shadow_map(
    light: &dyn Light,
    objects: &[Object],
    near: f32,
    far: f32,
    shadow_map_size: usize,
) -> ShadowMap {
    let lv_proj = light_view_proj(light, near, far);
    let mut shadow_map = ShadowMap::new(shadow_map_size, shadow_map_size, lv_proj);
    let size = shadow_map_size as f32;

    for obj in objects {
        if obj.is_light {
            continue;
        }

        let (model, _) = obj.transform.matrices();

        for &(i0, i1, i2) in &obj.mesh.faces {
            let w0 = (model * obj.mesh.vertices[i0].to_vec4()).to_vec3();
            let w1 = (model * obj.mesh.vertices[i1].to_vec4()).to_vec3();
            let w2 = (model * obj.mesh.vertices[i2].to_vec4()).to_vec3();

            // Returns None for vertices behind the light, outside depth range, or outside the x/y
            // frustum — keeps bounding boxes tight and avoids full-map iterations.
            let project = |world: Vec3| -> Option<(Vec2, f32)> {
                let clip = lv_proj * world.to_vec4();
                if clip.w <= 0.0 {
                    return None;
                }
                let ndc_x = clip.x / clip.w;
                let ndc_y = clip.y / clip.w;
                let ndc_z = clip.z / clip.w;
                let unit = -1.0..=1.0;
                if !unit.contains(&ndc_x) || !unit.contains(&ndc_y) || !unit.contains(&ndc_z) {
                    return None;
                }
                let sx = (ndc_x + 1.0) * 0.5 * size;
                let sy = (1.0 - ndc_y) * 0.5 * size;
                Some((Vec2::new(sx, sy), ndc_z))
            };

            // Skip any triangle where a vertex is behind the light or out of depth range.
            let (Some((p0, d0)), Some((p1, d1)), Some((p2, d2))) =
                (project(w0), project(w1), project(w2))
            else {
                continue;
            };

            let screen_tri = Triangle::screen_triangle(p0, p1, p2);
            let (min, max) = screen_tri.bounding_box();

            let min_x = (min.x.floor() as i32).max(0) as usize;
            let max_x = (max.x.ceil() as i32).min(shadow_map_size as i32 - 1) as usize;
            let min_y = (min.y.floor() as i32).max(0) as usize;
            let max_y = (max.y.ceil() as i32).min(shadow_map_size as i32 - 1) as usize;

            if min_x > max_x || min_y > max_y {
                continue;
            }

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    if let Some((bw0, bw1, bw2)) = screen_tri.contains_point(px, py) {
                        let depth = bw0 * d0 + bw1 * d1 + bw2 * d2;
                        shadow_map.write_depth(x, y, depth);
                    }
                }
            }
        }
    }

    shadow_map
}
