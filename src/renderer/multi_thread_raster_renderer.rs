use super::{TILE_SIZE, bin_triangles, draw_wireframe, prepare_object, rasterize_tile};
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::tile::make_tiles;
use rayon::prelude::*;
use std::sync::Arc;

pub struct MultiThreadRasterRenderer;

impl super::Renderer for MultiThreadRasterRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let triangles: Vec<_> = objects
            .iter()
            .flat_map(|obj| prepare_object(obj, width, height, camera, view, projection))
            .collect();

        let tiles = make_tiles(framebuffer.width, framebuffer.height, TILE_SIZE);
        let bins = bin_triangles(&triangles, &tiles, framebuffer.width);

        tiles
            .par_iter()
            .zip(bins.par_iter())
            .for_each(|(tile, tri_indices)| {
                rasterize_tile(
                    tile,
                    tri_indices,
                    &triangles,
                    camera,
                    lights,
                    framebuffer,
                    ambient,
                );
            });
    }

    fn render_wireframe(&self, objects: &[Object], camera: &Camera, framebuffer: &Framebuffer) {
        let width = framebuffer.width as f32;
        let height = framebuffer.height as f32;
        let view = camera.view_matrix();
        let projection = camera.projection_matrix();

        let triangles: Vec<_> = objects
            .iter()
            .flat_map(|obj| prepare_object(obj, width, height, camera, view, projection))
            .collect();

        draw_wireframe(&triangles, framebuffer);
    }
}
