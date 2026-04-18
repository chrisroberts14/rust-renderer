use crate::display::Display;
use std::sync::Arc;

pub struct VulkanDisplay {
    #[allow(dead_code)]
    window: Arc<dyn winit::window::Window>,
}

impl VulkanDisplay {
    pub fn new(window: Arc<dyn winit::window::Window>, _width: u32, _height: u32) -> Self {
        Self { window }
    }
}

impl Display for VulkanDisplay {
    fn present_cpu_frame(&self, _pixels: &[u8]) {
        todo!("Vulkan display not yet implemented")
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        todo!()
    }

    fn capture_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    fn release_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }

    fn request_redraw(&self) {
        todo!()
    }

    fn is_cursor_grabbed(&self) -> bool {
        todo!()
    }

    fn window(&self) -> Arc<dyn winit::window::Window> {
        self.window.clone()
    }
}
