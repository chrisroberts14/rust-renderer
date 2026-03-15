use crate::framebuffer::Framebuffer;

/// Used to declare submodules in the shapes module
pub mod line;
pub mod triangle;

pub trait Shape {
    /// Draws the shape into the frame buffer
    fn draw(&self, framebuffer: &mut Framebuffer);
}
