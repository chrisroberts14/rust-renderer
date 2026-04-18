use crate::display::Display;
use crate::renderer::vulkan::device::get_device_for_surface;
use std::cmp::{max, min};
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use winit::event_loop::EventLoop;
use winit::window::CursorGrabMode;

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
    swapchain: Arc<Swapchain>,
    #[allow(dead_code)]
    images: Vec<Arc<Image>>,
    cursor_grabbed: bool,
}

impl VulkanDisplay {
    pub fn new(
        window: Arc<dyn winit::window::Window>,
        event_loop: &EventLoop,
        width: u32,
        height: u32,
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

        let (swapchain, images) = Self::create_swapchain(&device, &surface, [width, height]);

        Self {
            window,
            instance,
            surface,
            device,
            queue,
            swapchain,
            images,
            cursor_grabbed: false,
        }
    }

    fn create_swapchain(
        device: &Arc<Device>,
        surface: &Arc<Surface>,
        extent: [u32; 2],
    ) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
        let caps = device
            .physical_device()
            .surface_capabilities(surface, Default::default())
            .unwrap();

        let image_count = match caps.max_image_count {
            None => max(2, caps.min_image_count),
            Some(limit) => min(max(2, caps.min_image_count), limit),
        };

        let (image_format, _color_space) = device
            .physical_device()
            .surface_formats(surface, Default::default())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: image_count,
                image_format,
                image_extent: extent,
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                present_mode: PresentMode::Fifo,
                ..Default::default()
            },
        )
        .unwrap()
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

    fn resize(&mut self, width: u32, height: u32) {
        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: [width, height],
                ..self.swapchain.create_info()
            })
            .unwrap();
        self.swapchain = new_swapchain;
        self.images = new_images;
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
