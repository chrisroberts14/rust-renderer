use std::sync::Arc;
use winit::window::CursorGrabMode;

pub struct CursorState {
    grabbed: bool,
}

impl CursorState {
    pub fn new() -> Self {
        Self { grabbed: false }
    }

    pub fn capture(
        &mut self,
        window: &dyn winit::window::Window,
    ) -> Result<(), Box<dyn std::error::Error>> {
        window.set_cursor_visible(false);
        if window.set_cursor_grab(CursorGrabMode::Confined).is_err() {
            window.set_cursor_grab(CursorGrabMode::Locked)?;
        }
        self.grabbed = true;
        Ok(())
    }

    pub fn release(
        &mut self,
        window: &dyn winit::window::Window,
    ) -> Result<(), Box<dyn std::error::Error>> {
        window.set_cursor_visible(true);
        window.set_cursor_grab(CursorGrabMode::None)?;
        self.grabbed = false;
        Ok(())
    }

    pub fn is_grabbed(&self) -> bool {
        self.grabbed
    }
}

pub trait Display {
    fn present_cpu_frame(&self, pixels: &[u8]);
    fn present_vk_frame(&self, _image: &Arc<vulkano::image::Image>) {
        panic!("Vulkan frame presentation not supported by this display backend");
    }
    fn present_gpu_frame(&self, _gpu_view: &wgpu::TextureView) {
        panic!("GPU frame presentation not supported by this display backend");
    }
    fn resize(&mut self, width: u32, height: u32);
    fn capture_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn release_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn request_redraw(&self);
    fn is_cursor_grabbed(&self) -> bool;
    fn window(&self) -> Arc<dyn winit::window::Window>;
}
