use std::sync::Arc;

pub trait Display {
    fn present_cpu_frame(&self, pixels: &[u8]);
    fn present_gpu_frame(&self, _gpu_view: &wgpu::TextureView, _overlay: Option<&[u8]>) {
        panic!("GPU frame presentation not supported by this display backend");
    }
    fn resize(&mut self, width: u32, height: u32);
    fn capture_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn release_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn request_redraw(&self);
    fn is_cursor_grabbed(&self) -> bool;
    fn window(&self) -> Arc<dyn winit::window::Window>;
}
