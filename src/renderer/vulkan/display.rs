use crate::display::Display;
use crate::renderer::vulkan::device::get_device_for_surface;
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use winit::event_loop::EventLoop;

pub struct VulkanDisplay {
    #[allow(dead_code)]
    window: Arc<dyn winit::window::Window>,
    #[allow(dead_code)]
    instance: Arc<Instance>,
    #[allow(dead_code)]
    surface: Arc<Surface>,
    #[allow(dead_code)]
    device: Arc<Device>,
    #[allow(dead_code)]
    queue: Arc<Queue>,
}

impl VulkanDisplay {
    pub fn new(
        window: Arc<dyn winit::window::Window>,
        event_loop: &EventLoop,
        _width: u32,
        _height: u32,
    ) -> Self {
        let lib = VulkanLibrary::new().unwrap();
        let required_exts = Surface::required_extensions(event_loop).unwrap();

        let instance = Instance::new(
            lib,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: required_exts,
                ..Default::default()
            },
        )
        .unwrap();

        // Safety: `window` is stored in `Self` alongside the surface, so window outlives surface.
        let surface = unsafe { Surface::from_window_ref(instance.clone(), &window) }.unwrap();

        let (device, queue) =
            get_device_for_surface(instance.clone(), &surface, Self::device_extensions()).unwrap();

        Self {
            window,
            instance,
            surface,
            device,
            queue,
        }
    }

    fn device_extensions() -> DeviceExtensions {
        DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        }
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
