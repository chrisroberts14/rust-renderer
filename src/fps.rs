use crate::framebuffer::Framebuffer;
use std::time::Instant;

fn digit_bitmap(ch: char) -> [u8; 5] {
    match ch {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0b000; 5],
    }
}

pub struct FpsCounter {
    last_time: Instant,
    frame_count: u32,
    pub fps: u32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }

    pub fn tick(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);

        if elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.last_time = now;
        }

        self.render_to_screen(framebuffer);
    }

    /// Write the current FPS to the top-left corner of the screen
    pub fn render_to_screen(&self, framebuffer: &mut Framebuffer) {
        let digits = self.fps.to_string();
        let color = [255u8, 255, 0, 255]; // yellow
        let scale = 3;
        let mut x_offset = 2;

        for ch in digits.chars() {
            let bitmap = digit_bitmap(ch);
            for (row, &bits) in bitmap.iter().enumerate() {
                for col in 0..3 {
                    if bits & (1 << (2 - col)) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                framebuffer.set_pixel(
                                    x_offset + col * scale + sx,
                                    2 + row * scale + sy,
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            x_offset += 4 * scale; // 3 wide + 1 gap
        }
    }
}
