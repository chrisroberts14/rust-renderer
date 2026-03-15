/// A structure representing a framebuffer with a specified width, height, and pixel data.
#[derive(Default)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    /// Create a new frame buffer with the given width and height, initializing the pixel data to zero.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height * 4], // Assuming RGBA format (4 bytes per pixel)
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        self.width = new_width;
        self.height = new_height;
        self.pixels.resize(new_width * new_height * 4, 0); // Resize pixel data accordingly
    }

    /// Set a single pixel
    pub fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return; // silently ignore out-of-bounds
        }
        let idx = (y * self.width + x) * 4;
        self.pixels[idx..idx + 4].copy_from_slice(&color);
    }

    /// Clear the framebuffer with a given color [R,G,B,A]
    pub fn clear(&mut self, color: [u8; 4]) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color);
        }
    }
}
