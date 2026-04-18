use crate::display::Display;
use pixels::{Pixels, SurfaceTexture};
use std::cell::RefCell;
use std::sync::Arc;
use winit::window::CursorGrabMode;

pub struct CpuDisplay {
    window: Arc<dyn winit::window::Window>,
    pixels: RefCell<Pixels<'static>>,
    cursor_grabbed: bool,
}

impl CpuDisplay {
    pub fn new(window: Arc<dyn winit::window::Window>, width: u32, height: u32) -> Self {
        // Pass window.clone() (owned Arc) so pixels creates a 'static surface.
        let surface = SurfaceTexture::new(width, height, window.clone());
        let pixels = Pixels::new(width, height, surface).expect("Failed to create pixel buffer");
        Self {
            window,
            pixels: RefCell::new(pixels),
            cursor_grabbed: false,
        }
    }
}

impl Display for CpuDisplay {
    fn present_cpu_frame(&self, pixel_bytes: &[u8]) {
        let mut pixels = self.pixels.borrow_mut();
        pixels.frame_mut().copy_from_slice(pixel_bytes);
        pixels.render().expect("Failed to render frame");
    }

    fn resize(&mut self, width: u32, height: u32) {
        let pixels = self.pixels.get_mut();
        pixels
            .resize_surface(width, height)
            .expect("Failed to resize surface");
        pixels
            .resize_buffer(width, height)
            .expect("Failed to resize buffer");
    }

    fn capture_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.window.set_cursor_visible(false);
        if self
            .window
            .set_cursor_grab(CursorGrabMode::Confined)
            .is_err()
        {
            self.window.set_cursor_grab(CursorGrabMode::Locked)?;
        }
        self.cursor_grabbed = true;
        Ok(())
    }

    fn release_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.window.set_cursor_visible(true);
        self.window.set_cursor_grab(CursorGrabMode::None)?;
        self.cursor_grabbed = false;
        Ok(())
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed
    }

    fn window(&self) -> Arc<dyn winit::window::Window> {
        self.window.clone()
    }
}
