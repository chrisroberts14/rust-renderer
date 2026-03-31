use super::{RendererChoice, prepare_render, rasterize_tile};
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::draw_wireframe;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use rayon::prelude::*;
use std::sync::Arc;

pub struct MultiThreadRasterRenderer {
    tile_size: usize,
}

impl MultiThreadRasterRenderer {
    pub fn new(tile_size: usize) -> Self {
        Self { tile_size }
    }
}

impl super::Renderer for MultiThreadRasterRenderer {
    fn renderer_choice(&self) -> RendererChoice {
        RendererChoice::MultiThreadRaster
    }

    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        let (triangles, tiles, bins) = prepare_render(objects, camera, framebuffer, self.tile_size);
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
        vec![
            ("Triangle Count", triangles.len().to_string()),
            ("Tile Count", tiles.len().to_string()),
        ]
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        let (triangles, _, _) = prepare_render(objects, camera, framebuffer, self.tile_size);
        draw_wireframe(&triangles, framebuffer);
        vec![("Triangle Count", triangles.len().to_string())]
    }

    fn increase_tile_count(&mut self, delta: usize) {
        self.tile_size += delta;
    }

    fn decrease_tile_count(&mut self, delta: usize) {
        if self.tile_size - delta >= 1 {
            self.tile_size -= delta;
        }
    }
}
