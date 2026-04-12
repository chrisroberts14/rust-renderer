use crate::framebuffer::Framebuffer;
use crate::geometry::triangle::Triangle;
use crate::renderer::tile::Tile;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use std::sync::Arc;

use super::PreparedTriangle;
use super::shade::shade;

/// Rasterizes all triangles assigned to a tile, clamping pixel iteration to the tile bounds.
pub(crate) fn rasterize_tile(
    tile: &Tile,
    triangle_indices: &[usize],
    triangles: &[PreparedTriangle],
    camera: &Camera,
    lights: &[Arc<dyn Light>],
    framebuffer: &Framebuffer,
    ambient: f32,
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
        let screen_tri = Triangle::screen_triangle(p0, p1, p2);

        // Clamp rasterization bounds to the tile (already backface-culled in prepare_object).
        let (min, max) = screen_tri.bounding_box();
        let min_x = (min.x.floor() as i32).max(tile_min_x);
        let max_x = (max.x.ceil() as i32).min(tile_max_x);
        let min_y = (min.y.floor() as i32).max(tile_min_y);
        let max_y = (max.y.ceil() as i32).min(tile_max_y);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                if let Some((w0, w1, w2)) = screen_tri.contains_point(px, py) {
                    // Interpolate depth and run the depth test before doing any shading work.
                    let depth = w0 * z0 + w1 * z1 + w2 * z2;
                    let ux = x as usize;
                    let uy = y as usize;

                    if framebuffer.test_and_set_depth(ux, uy, depth) {
                        let normal = (v0.normal * w0 + v1.normal * w1 + v2.normal * w2).normalise();
                        let world_pos = v0.world * w0 + v1.world * w1 + v2.world * w2;
                        let view_dir = (camera.position - world_pos).normalise();

                        let active_lights = if tri.is_light { &[] as &[_] } else { lights };
                        let [lr, lg, lb] =
                            shade(normal, world_pos, view_dir, active_lights, ambient);

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
                        let (r, g, b) = (f32::from(cr), f32::from(cg), f32::from(cb));

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
        let screen_tri = Triangle::screen_triangle(p0, p1, p2);
        framebuffer.draw_triangle_wireframe(&screen_tri);
    }
}
