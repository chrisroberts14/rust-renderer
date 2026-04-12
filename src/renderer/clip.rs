use crate::scenes::camera::Camera;

use super::Vert;

fn interpolate_vert(a: Vert, b: Vert, t: f32) -> Vert {
    Vert {
        cam: a.cam * (1.0 - t) + b.cam * t,
        world: a.world * (1.0 - t) + b.world * t,
        normal: a.normal * (1.0 - t) + b.normal * t,
        uv: a.uv * (1.0 - t) + b.uv * t,
    }
}

/// Clips a convex polygon against a single camera-space half-space defined by
/// `nx*x + ny*y + nz*z + d >= 0` (inside). Uses Sutherland-Hodgman.
fn clip_polygon_against_plane(polygon: &[Vert], nx: f32, ny: f32, nz: f32, d: f32) -> Vec<Vert> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let dist = |v: &Vert| nx * v.cam.x + ny * v.cam.y + nz * v.cam.z + d;
    let mut output = Vec::new();
    let n = polygon.len();
    for i in 0..n {
        let cur = polygon[i];
        let next = polygon[(i + 1) % n];
        let d_cur = dist(&cur);
        let d_next = dist(&next);
        if d_cur >= 0.0 {
            output.push(cur);
        }
        if (d_cur >= 0.0) != (d_next >= 0.0) {
            let t = d_cur / (d_cur - d_next);
            output.push(interpolate_vert(cur, next, t));
        }
    }
    output
}

/// Clips a triangle against all 6 frustum planes and returns the resulting triangles.
/// Eliminates any vertex that would project far outside the screen, preventing the
/// f32 precision failures that occur with very large screen-space triangles.
pub(super) fn clip_to_frustum(triangle: [Vert; 3], camera: &Camera) -> Vec<[Vert; 3]> {
    let tan_y = (camera.fov * 0.5).tan();
    let tan_x = camera.aspect_ratio * tan_y;
    // Each plane: (nx, ny, nz, d) — inside when nx*x + ny*y + nz*z + d >= 0 in camera space.
    let planes: [(f32, f32, f32, f32); 6] = [
        (0.0, 0.0, -1.0, -camera.near), // near:   z <= -near
        (0.0, 0.0, 1.0, camera.far),    // far:    z >= -far
        (-1.0, 0.0, -tan_x, 0.0),       // right:  x <= tan_x * (-z)
        (1.0, 0.0, -tan_x, 0.0),        // left:   x >= -tan_x * (-z)
        (0.0, -1.0, -tan_y, 0.0),       // top:    y <= tan_y * (-z)
        (0.0, 1.0, -tan_y, 0.0),        // bottom: y >= -tan_y * (-z)
    ];
    let mut polygon: Vec<Vert> = triangle.to_vec();
    for (nx, ny, nz, d) in planes {
        polygon = clip_polygon_against_plane(&polygon, nx, ny, nz, d);
        if polygon.is_empty() {
            return Vec::new();
        }
    }
    if polygon.len() < 3 {
        return Vec::new();
    }
    // Fan-triangulate the clipped polygon from vertex 0.
    (1..polygon.len() - 1)
        .map(|i| [polygon[0], polygon[i], polygon[i + 1]])
        .collect()
}
