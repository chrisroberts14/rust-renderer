use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::prepare::prepare_render;
use crate::renderer::rasterize::{ShadingContext, draw_wireframe, rasterize_tile};
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

    pub fn increase_tile_count(&mut self, delta: usize) {
        self.tile_size += delta;
    }

    pub fn decrease_tile_count(&mut self, delta: usize) {
        if self.tile_size - delta >= 1 {
            self.tile_size -= delta;
        }
    }
}

impl super::Renderer for MultiThreadRasterRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        let (triangles, tiles, bins) = prepare_render(objects, camera, framebuffer, self.tile_size);
        let shading = ShadingContext {
            lights,
            shadow_maps: &[],
            ambient,
        };
        tiles
            .par_iter()
            .zip(bins.par_iter())
            .for_each(|(tile, tri_indices)| {
                rasterize_tile(tile, tri_indices, &triangles, camera, &shading, framebuffer);
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
}
