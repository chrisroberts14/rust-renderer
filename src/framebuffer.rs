use crate::geometry::triangle::Triangle;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::texture::Texture;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// A structure representing a framebuffer with a specified width, height, and pixel data.
#[derive(Default)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<AtomicU32>,
    pub depth: Vec<AtomicU32>,
}

impl Framebuffer {
    /// Create a new frame buffer with the given width and height, initializing the pixel data to zero.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: (0..width * height).map(|_| AtomicU32::new(0)).collect(),
            depth: (0..width * height)
                .map(|_| AtomicU32::new(f32::INFINITY.to_bits()))
                .collect(),
        }
    }

    /// Set a single pixel
    pub fn set_pixel(&self, x: usize, y: usize, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return; // silently ignore out-of-bounds
        }
        let idx = y * self.width + x;
        self.pixels[idx].store(
            u32::from_ne_bytes(color),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Clear the framebuffer to be all black
    pub fn clear(&self) {
        let packed = u32::from_ne_bytes([0, 0, 0, 255]);
        for p in &self.pixels {
            p.store(packed, Ordering::Relaxed);
        }
        for d in &self.depth {
            d.store(f32::INFINITY.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn test_and_set_depth(&self, x: usize, y: usize, depth: f32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = y * self.width + x;
        let depth_bits = depth.to_bits();
        let mut current = self.depth[idx].load(std::sync::atomic::Ordering::Relaxed);
        loop {
            if depth_bits >= current {
                return false;
            }
            match self.depth[idx].compare_exchange_weak(
                current,
                depth_bits,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Returns the pixel data as a flat byte slice.
    /// Safe because AtomicU32 has the same layout as u32, and we pack pixels as from_ne_bytes.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, self.pixels.len() * 4)
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.pixels = (0..new_width * new_height)
            .map(|_| AtomicU32::new(0))
            .collect();
        self.depth = (0..new_width * new_height)
            .map(|_| AtomicU32::new(f32::INFINITY.to_bits()))
            .collect();
    }

    pub fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let mut x0 = x0;
        let mut y0 = y0;
        let mut x1 = x1;
        let mut y1 = y1;

        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = (y1 - y0).abs();
        let y_step = if y0 < y1 { 1 } else { -1 };

        let mut error = dx / 2;
        let mut y = y0;

        for x in x0..=x1 {
            if steep {
                self.set_pixel(y as usize, x as usize, [255, 255, 255, 255]);
            } else {
                self.set_pixel(x as usize, y as usize, [255, 255, 255, 255]);
            }
            error -= dy;
            if error < 0 {
                y += y_step;
                error += dx;
            }
        }
    }

    pub fn draw_triangle_wireframe(&self, triangle: &Triangle) {
        let p0 = triangle.v0;
        let p1 = triangle.v1;
        let p2 = triangle.v2;

        self.draw_line(p0.x as i32, p0.y as i32, p1.x as i32, p1.y as i32);
        self.draw_line(p1.x as i32, p1.y as i32, p2.x as i32, p2.y as i32);
        self.draw_line(p2.x as i32, p2.y as i32, p0.x as i32, p0.y as i32);
    }

    /// Draw the skybox infinitely far away
    pub fn draw_skybox(&self, texture: &Texture, camera: &Camera) {
        let tan_half_fov = (camera.fov / 2.0).tan();
        let right = camera.right();
        let up = camera.up();
        let forward = camera.forward();

        (0..self.height).into_par_iter().for_each(|y| {
            let ndc_y = 1.0 - (y as f32 + 0.5) / self.height as f32 * 2.0;
            let ray_y = ndc_y * tan_half_fov;
            for x in 0..self.width {
                let ndc_x = (x as f32 + 0.5) / self.width as f32 * 2.0 - 1.0;

                let ray_x = ndc_x * camera.aspect_ratio * tan_half_fov;

                // Build view-space ray and rotate into world space using camera axes
                let view_ray = Vec3::new(ray_x, ray_y, -1.0);
                let world_ray =
                    (right * view_ray.x + up * view_ray.y + forward * (-view_ray.z)).normalise();

                // Convert world direction to equirectangular UV
                let u = world_ray.z.atan2(world_ray.x) / (2.0 * std::f32::consts::PI) + 0.5;
                let v = 0.5 - world_ray.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI;

                let color = texture.sample(u, v);
                self.set_pixel(x, y, color);
            }
        });
    }
}
