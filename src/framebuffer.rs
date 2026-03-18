use std::sync::atomic::{AtomicU8, AtomicU32};

/// A structure representing a framebuffer with a specified width, height, and pixel data.
#[derive(Default)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<AtomicU8>,
    pub depth: Vec<AtomicU32>,
}

impl Framebuffer {
    /// Create a new frame buffer with the given width and height, initializing the pixel data to zero.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: (0..width * height * 4).map(|_| AtomicU8::new(0)).collect(),
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
        let idx = (y * self.width + x) * 4;
        self.pixels[idx..idx + 4]
            .iter()
            .zip(color.iter())
            .for_each(|(p, c)| {
                p.store(*c, std::sync::atomic::Ordering::Relaxed);
            });
    }

    /// Clear the framebuffer with a given color [R,G,B,A]
    pub fn clear(&self, color: [u8; 4]) {
        self.pixels.chunks_exact(4).for_each(|chunk| {
            chunk.iter().zip(color.iter()).for_each(|(p, c)| {
                p.store(*c, std::sync::atomic::Ordering::Relaxed);
            })
        });
        self.depth.iter().for_each(|d| {
            d.store(
                f32::INFINITY.to_bits(),
                std::sync::atomic::Ordering::Relaxed,
            )
        });
    }

    pub fn test_and_set_depth(&self, x: usize, y: usize, depth: f32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = y * self.width + x;
        let depth_bits = depth.to_bits();
        let current_depth_bits = self.depth[idx].load(std::sync::atomic::Ordering::Relaxed);
        if depth_bits < current_depth_bits {
            self.depth[idx].store(depth_bits, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Returns the pixel data as a flat byte slice.
    /// Safe because AtomicU8 is guaranteed to have the same layout as u8.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, self.pixels.len()) }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.pixels = (0..new_width * new_height * 4)
            .map(|_| AtomicU8::new(0))
            .collect();
        self.depth = (0..new_width * new_height)
            .map(|_| AtomicU32::new(f32::INFINITY.to_bits()))
            .collect();
    }
}
