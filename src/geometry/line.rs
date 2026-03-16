use crate::{framebuffer::Framebuffer, maths::vec2::Vec2};

pub struct Line {
    pub v0: Vec2,
    pub v1: Vec2,
}

impl Line {
    pub fn new(v0: Vec2, v1: Vec2) -> Self {
        Self { v0, v1 }
    }

    pub fn get_intermediary_pixels(&self) -> Vec<(usize, usize)> {
        let mut pixels = Vec::new();

        let dx = (self.v1.x - self.v0.x).abs();
        let dy = (self.v1.y - self.v0.y).abs();

        let sx = if self.v0.x < self.v1.x { 1.0 } else { -1.0 };
        let sy = if self.v0.y < self.v1.y { 1.0 } else { -1.0 };

        let mut err = if dx > dy { dx } else { -dy } / 2.0;
        let mut x = self.v0.x;
        let mut y = self.v0.y;

        loop {
            pixels.push((x as usize, y as usize));
            if x == self.v1.x && y == self.v1.y {
                break;
            }
            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
        pixels
    }

    pub fn draw(&self, framebuffer: &mut Framebuffer, colour: [u8; 4]) {
        let pixels = self.get_intermediary_pixels();
        for (x, y) in pixels {
            framebuffer.set_pixel(x, y, colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horizontal_line() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 0.0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (1, 0), (2, 0), (3, 0)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_vertical_line() {
        let line = Line::new(Vec2::new(2.0, 1.0), Vec2::new(2.0, 4.0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(2, 1), (2, 2), (2, 3), (2, 4)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_diagonal_line() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (1, 1), (2, 2), (3, 3)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_reverse_diagonal_line() {
        let line = Line::new(Vec2::new(3.0, 3.0), Vec2::new(0.0, 0.0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(3, 3), (2, 2), (1, 1), (0, 0)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_steep_line() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 5.0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (0, 1), (1, 2), (1, 3), (2, 4), (2, 5)];
        assert_eq!(pixels, expected);
    }
}
