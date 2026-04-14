use crate::geometry::triangle::Triangle;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::texture::Texture;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

const INFINITY_BITS: u32 = f32::INFINITY.to_bits();
const COLOR_WHITE: [u8; 4] = [255, 255, 255, 255];
const COLOR_BLACK: [u8; 4] = [0, 0, 0, 255];

#[derive(Default, Debug)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pixels: Vec<AtomicU32>,
    depth: Vec<AtomicU32>,
}

/// Custom clone so we can copy scenes
impl Clone for Framebuffer {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            pixels: self
                .pixels
                .iter()
                .map(|a| AtomicU32::new(a.load(Ordering::Relaxed)))
                .collect(),
            depth: self
                .depth
                .iter()
                .map(|a| AtomicU32::new(a.load(Ordering::Relaxed)))
                .collect(),
        }
    }
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self {
            width,
            height,
            pixels: (0..n).map(|_| AtomicU32::new(0)).collect(),
            depth: (0..n).map(|_| AtomicU32::new(INFINITY_BITS)).collect(),
        }
    }

    pub fn set_pixel(&self, x: usize, y: usize, color: [u8; 4]) {
        if !self.in_bounds(x, y) {
            return;
        }
        self.pixels[self.pixel_idx(x, y)].store(u32::from_ne_bytes(color), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        let packed = u32::from_ne_bytes(COLOR_BLACK);
        for (p, d) in self.pixels.iter().zip(self.depth.iter()) {
            p.store(packed, Ordering::Relaxed);
            d.store(INFINITY_BITS, Ordering::Relaxed);
        }
    }

    pub fn test_and_set_depth(&self, x: usize, y: usize, depth: f32) -> bool {
        if !self.in_bounds(x, y) {
            return false;
        }
        let idx = self.pixel_idx(x, y);
        let depth_bits = depth.to_bits();
        let mut current = self.depth[idx].load(Ordering::Relaxed);
        loop {
            if depth_bits >= current {
                return false;
            }
            match self.depth[idx].compare_exchange_weak(
                current,
                depth_bits,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Safe because AtomicU32 has the same layout as u32, and we pack pixels as from_ne_bytes.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, self.pixels.len() * 4)
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        *self = Self::new(new_width, new_height);
    }

    pub fn draw_line(&self, mut x0: i32, mut y0: i32, mut x1: i32, mut y1: i32) {
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
                self.set_pixel(y as usize, x as usize, COLOR_WHITE);
            } else {
                self.set_pixel(x as usize, y as usize, COLOR_WHITE);
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
                let world_ray = right * view_ray.x + up * view_ray.y + forward * (-view_ray.z);

                // Convert world direction to equirectangular UV
                let u = world_ray.z.atan2(world_ray.x) / (2.0 * std::f32::consts::PI) + 0.5;
                let v = 0.5 - world_ray.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI;

                let color = texture.sample(u, v);
                self.set_pixel(x, y, color);
            }
        });
    }

    #[inline]
    fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    #[inline]
    fn pixel_idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    #[cfg(test)]
    fn get_pixel(&self, x: usize, y: usize) -> [u8; 4] {
        self.pixels[self.pixel_idx(x, y)]
            .load(Ordering::Relaxed)
            .to_ne_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_bounds() {
        let fb = Framebuffer::new(256, 256);

        // In bounds
        assert!(fb.in_bounds(0, 0)); // Top left
        assert!(fb.in_bounds(255, 255)); // Bottom right
        assert!(fb.in_bounds(128, 128)); // Rough centre

        // Out of bounds
        assert!(!fb.in_bounds(256, 255));
        assert!(!fb.in_bounds(255, 256));
        assert!(!fb.in_bounds(256, 256));
    }

    #[test]
    fn test_pixel_idx() {
        let fb = Framebuffer::new(256, 256);
        assert_eq!(fb.pixel_idx(0, 0), 0);
        assert_eq!(fb.pixel_idx(0, 255), 65280);
        assert_eq!(fb.pixel_idx(255, 0), 255);
        assert_eq!(fb.pixel_idx(255, 255), 65535);
    }

    #[test]
    fn test_set_pixel() {
        let fb = Framebuffer::new(256, 256);
        fb.set_pixel(0, 0, COLOR_WHITE);
        assert_eq!(fb.get_pixel(0, 0), COLOR_WHITE);
    }

    #[test]
    fn test_set_pixel_fails_when_oob() {
        let fb = Framebuffer::new(256, 256);
        fb.set_pixel(1000, 1000, COLOR_WHITE);
        for x in 0..256 {
            for y in 0..256 {
                assert_eq!(fb.get_pixel(x, y), [0, 0, 0, 0]);
            }
        }
    }
}
