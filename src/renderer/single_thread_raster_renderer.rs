use super::{TILE_SIZE, bin_triangles, draw_wireframe, prepare_object, rasterize_tile};
/// Single threaded version of the raster renderer. Used for testing and debugging,
/// as it is not as performant as the multi-threaded version.
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::tile::make_tiles;
use std::sync::Arc;

pub struct SingleThreadRasterRenderer;

impl super::Renderer for SingleThreadRasterRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
    ) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let triangles: Vec<_> = objects
            .iter()
            .flat_map(|obj| prepare_object(obj, width, height, view, projection, camera.near))
            .collect();

        let tiles = make_tiles(framebuffer.width, framebuffer.height, TILE_SIZE);
        let bins = bin_triangles(&triangles, &tiles, framebuffer.width);

        tiles
            .iter()
            .zip(bins.iter())
            .for_each(|(tile, tri_indices)| {
                rasterize_tile(tile, tri_indices, &triangles, camera, lights, framebuffer);
            });
    }

    fn render_wireframe(&self, objects: &[Object], camera: &Camera, framebuffer: &Framebuffer) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let triangles: Vec<_> = objects
            .iter()
            .flat_map(|obj| prepare_object(obj, width, height, view, projection, camera.near))
            .collect();

        draw_wireframe(&triangles, framebuffer);
    }
}
