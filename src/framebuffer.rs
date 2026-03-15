/// A structure representing a framebuffer with a specified width, height, and pixel data.
#[derive(Default)]
pub struct Framebuffer{
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>
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

    /// Set the pixel at the specified (x, y) coordinates to the given RGBA color values.
    /// The color values should be in the range of 0 to 255.
    pub fn set_pixel(&mut self, x: usize, y: usize, colour: [u8;4]) {
        let index = (y * self.width + x) * 4; // Calculate the index for RGBA format
        self.pixels[index] = colour[0];     // Red
        self.pixels[index + 1] = colour[1]; // Green
        self.pixels[index + 2] = colour[2]; // Blue
        self.pixels[index + 3] = colour[3]; // Alpha
    }

    /// Set all pixels in the framebuffer to the specified RGBA color values.
    pub fn clear(&mut self, colour: [u8;4]) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i] = colour[0];     // Red
            self.pixels[i + 1] = colour[1]; // Green
            self.pixels[i + 2] = colour[2]; // Blue
            self.pixels[i + 3] = colour[3]; // Alpha
        }
    }
}
