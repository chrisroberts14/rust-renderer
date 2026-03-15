use crate::{
    framebuffer::Framebuffer,
    maths::vec2::Vec2,
    shapes::{Shape, line::Line},
};

#[allow(dead_code)]
pub struct Triangle {
    pub v0: Vec2,
    pub v1: Vec2,
    pub v2: Vec2,
}

#[allow(dead_code)]
impl Triangle {
    pub fn new(v0: Vec2, v1: Vec2, v2: Vec2) -> Self {
        Self { v0, v1, v2 }
    }

    // Returns all edges as Lines
    pub fn edges(&self) -> [Line; 3] {
        [
            Line::new(self.v0, self.v1),
            Line::new(self.v1, self.v2),
            Line::new(self.v2, self.v0),
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
