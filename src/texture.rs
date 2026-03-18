#[allow(dead_code)]
struct Texture {
    pub width: u32,
    pub height: u32,
    rgba: Vec<u8>,
}

#[allow(dead_code)]
impl Texture {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let u = u.fract().abs();
        let v = v.fract().abs();

        let x = (u * self.width as f32) as u32;
        let y = (v * self.height as f32) as u32;

        // Avoid any out of bounds issues
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);

        let idx = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[idx],
            self.rgba[idx + 1],
            self.rgba[idx + 2],
            self.rgba[idx + 3],
        ]
    }
}
