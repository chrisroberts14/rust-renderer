use crate::{framebuffer::Framebuffer, shapes::Shape};

pub struct Line {
    pub v0: (usize, usize),
    pub v1: (usize, usize),
}

impl Line {
    pub fn new(v0: (usize, usize), v1: (usize, usize)) -> Self {
        Self { v0, v1 }
    }

    pub fn get_intermediary_pixels(&self) -> Vec<(usize, usize)> {
        let mut pixels = Vec::new();

        let (x0, y0) = (self.v0.0 as isize, self.v0.1 as isize);
        let (x1, y1) = (self.v1.0 as isize, self.v1.1 as isize);

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = if dx > dy { dx } else { -dy } / 2;
        let mut x = x0;
        let mut y = y0;

        loop {
            pixels.push((x as usize, y as usize));
            if x == x1 && y == y1 {
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
}

impl Shape for Line {
    fn draw(&self, framebuffer: &mut Framebuffer) {
        let colour = [0, 255, 0, 255];
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
        let line = Line::new((0, 0), (3, 0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (1, 0), (2, 0), (3, 0)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_vertical_line() {
        let line = Line::new((2, 1), (2, 4));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(2, 1), (2, 2), (2, 3), (2, 4)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_diagonal_line() {
        let line = Line::new((0, 0), (3, 3));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (1, 1), (2, 2), (3, 3)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_reverse_diagonal_line() {
        let line = Line::new((3, 3), (0, 0));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(3, 3), (2, 2), (1, 1), (0, 0)];
        assert_eq!(pixels, expected);
    }

    #[test]
    fn test_steep_line() {
        let line = Line::new((0, 0), (2, 5));
        let pixels = line.get_intermediary_pixels();
        let expected = vec![(0, 0), (0, 1), (1, 2), (1, 3), (2, 4), (2, 5)];
        assert_eq!(pixels, expected);
    }
}
