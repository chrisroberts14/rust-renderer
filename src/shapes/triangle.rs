use crate::{
    framebuffer::Framebuffer,
    shapes::{Shape, line::Line},
};

pub struct Triangle {
    pub v0: (usize, usize),
    pub v1: (usize, usize),
    pub v2: (usize, usize),
}

impl Triangle {
    pub fn new(v0: (usize, usize), v1: (usize, usize), v2: (usize, usize)) -> Self {
        Self { v0, v1, v2 }
    }

    // Returns all edges as Lines
    pub fn edges(&self) -> [Line; 3] {
        [
            Line::new((self.v0.0, self.v0.1), (self.v1.0, self.v1.1)),
            Line::new((self.v1.0, self.v1.1), (self.v2.0, self.v2.1)),
            Line::new((self.v2.0, self.v2.1), (self.v0.0, self.v0.1)),
        ]
    }
}

impl Shape for Triangle {
    fn draw(&self, framebuffer: &mut Framebuffer) {
        for edge in self.edges() {
            edge.draw(framebuffer);
        }
    }
}
