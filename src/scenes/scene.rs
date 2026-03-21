use crate::geometry::cube::Cube;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::renderer::{Renderer, TILE_SIZE, bin_triangles};
use crate::scenes::camera::Camera;
use crate::scenes::material::Material;
use crate::scenes::pointlight::PointLight;
use crate::scenes::texture::Texture;
use crate::tile::make_tiles;
use crate::{framebuffer::Framebuffer, geometry::object::Object};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use rayon::prelude::*;

pub struct SceneSettings {
    pub render_lights: bool,
    pub wire_frame_mode: bool,
}

impl SceneSettings {
    pub fn new() -> Self {
        Self {
            render_lights: false,
            wire_frame_mode: false,
        }
    }

    pub fn toggle_render_lights(&mut self) {
        self.render_lights = !self.render_lights;
    }

    pub fn toggle_wire_frame_mode(&mut self) {
        self.wire_frame_mode = !self.wire_frame_mode;
    }
}

pub struct Scene {
    pub objects: Arc<RwLock<Vec<Object>>>,
    pub framebuffer: Framebuffer,
    pub camera: Camera,
    pub lights: Vec<PointLight>,
    pub settings: SceneSettings,
    pub skybox: Option<Texture>,
}

impl Scene {
    pub fn new(width: f32, height: f32, objects: Vec<Object>, lights: Vec<PointLight>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(objects)),
            framebuffer: Framebuffer::new(width as usize, height as usize),
            camera: Camera::new(width, height),
            lights,
            settings: SceneSettings::new(),
            skybox: None,
        }
    }

    /// Spawn a thread that continuously updates object transforms.
    /// Returns the join handle and a shutdown flag — set the flag to false and join the handle to stop the thread cleanly.
    pub fn spawn_update_thread(&self) -> (thread::JoinHandle<()>, Arc<AtomicBool>) {
        let objects = Arc::clone(&self.objects);
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = Arc::clone(&running);
        let handle = thread::spawn(move || {
            while thread_running.load(Ordering::Relaxed) {
                {
                    let mut objs = objects.write().unwrap();
                    for object in objs.iter_mut() {
                        object.transform.rotation.x += 0.01;
                        object.transform.rotation.x %= 2.0 * std::f32::consts::PI;
                        object.transform.rotation.y += 0.01;
                        object.transform.rotation.y %= 2.0 * std::f32::consts::PI;
                    }
                } // write lock dropped here
                thread::sleep(Duration::from_millis(16));
            }
        });
        (handle, running)
    }

    pub fn render_objects(&mut self) {
        let fb_width = self.framebuffer.width as f32;
        let fb_height = self.framebuffer.height as f32;

        // Geometry pass: transform, clip, project, and backface-cull all objects.
        let objects = self.objects.read().unwrap();

        let view = self.camera.view_matrix();
        let projection = self.camera.projection_matrix();

        let triangles: Vec<_> = objects
            .iter()
            .flat_map(|obj| Renderer::prepare_object(obj, fb_width, fb_height, view, projection, self.camera.near))
            .collect();

        if self.settings.wire_frame_mode {
            Renderer::draw_wireframe(&triangles, &self.framebuffer);
            return;
        }

        // Binning + rasterization pass.
        let tiles = make_tiles(self.framebuffer.width, self.framebuffer.height, TILE_SIZE);
        let bins = bin_triangles(&triangles, &tiles);
        tiles.par_iter().zip(bins.par_iter()).for_each(|(tile, tri_indices)| {
                Renderer::rasterize_tile(tile, tri_indices, &triangles, &self.camera, &self.lights, &self.framebuffer);
        });
    }

    /// Renders small box representations of each point light for debugging.
    /// Light boxes are rendered unlit so their colour always matches the light colour.
    pub fn render_lights(&mut self) {
        let fb_width = self.framebuffer.width as f32;
        let fb_height = self.framebuffer.height as f32;
        let view = self.camera.view_matrix();
        let projection = self.camera.projection_matrix();

        let triangles: Vec<_> = self
            .lights
            .iter()
            .flat_map(|light| {
                let colour = [
                    (light.colour[0] * 255.0) as u8,
                    (light.colour[1] * 255.0) as u8,
                    (light.colour[2] * 255.0) as u8,
                    255,
                ];
                let light_box = Object::new(
                    Cube::mesh(1.0),
                    Transform {
                        position: light.position,
                        rotation: Vec3::new(0.0, 0.0, 0.0),
                        scale: Vec3::new(0.1, 0.1, 0.1),
                    },
                    Material::Color(colour),
                );
                Renderer::prepare_object(&light_box, fb_width, fb_height, view, projection, self.camera.near)
            })
            .collect();

        if self.settings.wire_frame_mode {
            Renderer::draw_wireframe(&triangles, &self.framebuffer);
            return;
        }

        let tiles = make_tiles(self.framebuffer.width, self.framebuffer.height, TILE_SIZE);
        let bins = bin_triangles(&triangles, &tiles);
        for (tile, tri_indices) in tiles.iter().zip(bins.iter()) {
            // Pass empty lights — light boxes should appear unlit.
            Renderer::rasterize_tile(
                tile,
                tri_indices,
                &triangles,
                &self.camera,
                &[],
                &self.framebuffer,
            );
        }
    }

    /// Helper method to render the whole scene
    pub fn render_scene(&mut self) {
        self.framebuffer.clear();
        if let Some(skybox) = &self.skybox {
            self.framebuffer.draw_skybox(skybox, &self.camera);
        }
        self.render_objects();
        if self.settings.render_lights {
            self.render_lights();
        }
    }
}
