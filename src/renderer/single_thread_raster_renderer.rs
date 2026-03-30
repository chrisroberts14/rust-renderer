/// Single threaded version of the raster renderer. Used for testing and debugging,
/// as it is not as performant as the multi-threaded version.
use super::{prepare_render, rasterize_tile, RendererChoice};
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::{RenderStats, draw_wireframe};
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use std::sync::Arc;

pub struct SingleThreadRasterRenderer {
    tile_size: usize,
}

impl SingleThreadRasterRenderer {
    pub fn new(tile_size: usize) -> Self {
        Self { tile_size }
    }
}

impl super::Renderer for SingleThreadRasterRenderer {
    fn renderer_choice(&self) -> RendererChoice {
        RendererChoice::SingleThreadRaster
    }

    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> RenderStats {
        let (triangles, tiles, bins) = prepare_render(objects, camera, framebuffer, self.tile_size);
        tiles
            .iter()
            .zip(bins.iter())
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
        RenderStats {
            triangle_count: triangles.len(),
            tile_count: tiles.len(),
        }
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> RenderStats {
        let (triangles, _, _) = prepare_render(objects, camera, framebuffer, self.tile_size);
        draw_wireframe(&triangles, framebuffer);
        RenderStats {
            triangle_count: triangles.len(),
            tile_count: 0,
        }
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
