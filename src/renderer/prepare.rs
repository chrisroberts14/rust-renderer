use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::geometry::triangle::Triangle;
use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::renderer::tile::{Tile, make_tiles};
use crate::scenes::camera::Camera;

use super::clip::clip_to_frustum;
use super::{PreparedTriangle, Vert};

/// Shared setup for raster rendering: transforms objects into prepared triangles, builds the tile
/// grid, and bins triangles into tiles. Both raster renderers call this, then differ only in
/// whether they dispatch tile rasterization with `iter` or `par_iter`.
pub(crate) fn prepare_render(
    objects: &[Object],
    camera: &Camera,
    framebuffer: &Framebuffer,
    tile_size: usize,
) -> (Vec<PreparedTriangle>, Vec<Tile>, Vec<Vec<usize>>) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let view = camera.view_matrix();
    let projection = camera.projection_matrix();

    let triangles: Vec<_> = objects
        .iter()
        .flat_map(|obj| prepare_object(obj, width, height, camera, view, projection))
        .collect();

    let tiles = make_tiles(framebuffer.width, framebuffer.height, tile_size);
    let bins = bin_triangles(&triangles, &tiles, framebuffer.width, tile_size);
    (triangles, tiles, bins)
}

/// Geometry pass: transforms, clips, projects, and backface-culls all faces of an object.
/// Returns a flat list of screen-ready triangles with no framebuffer writes.
pub(super) fn prepare_object(
    object: &Object,
    width: f32,
    height: f32,
    camera: &Camera,
    camera_view_mat: Mat4,
    camera_projection_mat: Mat4,
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

        // Clip against all 6 frustum planes. May produce 0 or more triangles.
        for [v0, v1, v2] in clip_to_frustum([v0, v1, v2], camera) {
            // Project camera-space positions to 2D screen coordinates.
            // z values are NDC depth, kept for depth interpolation during rasterization.
            let ((p0, z0), (p1, z1), (p2, z2)) =
                Triangle::new(v0.cam, v1.cam, v2.cam).project(camera_projection_mat, width, height);

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
                is_light: object.is_light,
            });
        }
    }

    triangles
}

/// Binning pass: assigns each triangle to every tile whose bounds overlap its screen bounding box.
/// Returns one `Vec<usize>` per tile, containing indices into `triangles`.
pub(super) fn bin_triangles(
    triangles: &[PreparedTriangle],
    tiles: &[Tile],
    screen_width: usize,
    tile_size: usize,
) -> Vec<Vec<usize>> {
    let tiles_per_row = screen_width.div_ceil(tile_size);

    let mut bins: Vec<Vec<usize>> = vec![Vec::new(); tiles.len()];

    // Clamp to valid tile grid
    let max_tile_x = tiles_per_row - 1;
    let max_tile_y = (tiles.len() / tiles_per_row) - 1;

    for (tri_idx, tri) in triangles.iter().enumerate() {
        let [p0, p1, p2] = tri.screen;

        // Compute triangle screen-space bounding box
        let min_x = p0.x.min(p1.x).min(p2.x).floor().max(0.0) as usize;
        let max_x = p0.x.max(p1.x).max(p2.x).ceil().max(0.0) as usize;
        let min_y = p0.y.min(p1.y).min(p2.y).floor().max(0.0) as usize;
        let max_y = p0.y.max(p1.y).max(p2.y).ceil().max(0.0) as usize;

        // Convert pixel bounds → tile indices
        let tile_min_x = min_x / tile_size;
        let tile_max_x = max_x / tile_size;
        let tile_min_y = min_y / tile_size;
        let tile_max_y = max_y / tile_size;

        let tile_min_x = tile_min_x.min(max_tile_x);
        let tile_max_x = tile_max_x.min(max_tile_x);
        let tile_min_y = tile_min_y.min(max_tile_y);
        let tile_max_y = tile_max_y.min(max_tile_y);

        // Assign triangle to overlapping tiles
        for ty in tile_min_y..=tile_max_y {
            let row_start = ty * tiles_per_row;
            for tx in tile_min_x..=tile_max_x {
                let tile_idx = row_start + tx;
                bins[tile_idx].push(tri_idx);
            }
        }
    }

    bins
}
