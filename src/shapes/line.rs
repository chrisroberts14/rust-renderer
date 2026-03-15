pub struct Line {
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Line {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn get_intermediary_pixels(&self) -> Vec<(usize, usize)> {
        let mut pixels = Vec::new();

        let mut x0 = self.x1 as isize;
        let mut y0 = self.y1 as isize;
        let x1 = self.x2 as isize;
        let y1 = self.y2 as isize;

        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            pixels.push((x0 as usize, y0 as usize));
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }

        pixels
    }
}
