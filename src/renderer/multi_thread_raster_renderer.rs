use super::{prepare_render, rasterize_tile};
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::RenderStats;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
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
    ) -> RenderStats {
        let (triangles, tiles, bins) = prepare_render(objects, camera, framebuffer);
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
        RenderStats {
            triangle_count: triangles.len(),
            tile_count: tiles.len(),
        }
    }
}
