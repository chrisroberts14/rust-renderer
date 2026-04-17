/// Single threaded version of the raster renderer. Used for testing and debugging,
/// as it is not as performant as the multi-threaded version.
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::Renderer;
use crate::renderer::prepare::prepare_render;
use crate::renderer::rasterize::{ShadingContext, draw_wireframe, rasterize_tile};
use crate::renderer::shadow_map::build_shadow_map;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use std::sync::Arc;

// Smaller than the multi-threaded renderer's map for faster single-core debug builds.
const SHADOW_MAP_SIZE: usize = 128;

pub struct SingleThreadRasterRenderer {
    tile_size: usize,
    shadow_map_size: usize,
}

impl SingleThreadRasterRenderer {
    pub fn new(tile_size: usize) -> Self {
        Self {
            tile_size,
            shadow_map_size: SHADOW_MAP_SIZE,
        }
    }

    pub fn with_shadow_map_size(mut self, size: usize) -> Self {
        self.shadow_map_size = size;
        self
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

impl Renderer for SingleThreadRasterRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        let shadow_maps: Vec<_> = lights
            .iter()
            .map(|light| {
                build_shadow_map(
                    light.as_ref(),
                    objects,
                    camera.near,
                    camera.far,
                    self.shadow_map_size,
                )
            })
            .collect();

        let (triangles, tiles, bins) = prepare_render(objects, camera, framebuffer, self.tile_size);
        let shading = ShadingContext {
            lights,
            shadow_maps: &shadow_maps,
            ambient,
        };
        tiles
            .iter()
            .zip(bins.iter())
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
