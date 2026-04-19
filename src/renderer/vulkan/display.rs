use crate::display::{CursorState, Display};
use crate::renderer::vulkan::device::get_device_for_surface;
use crate::renderer::vulkan::overlay::VulkanOverlay;
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage, CopyBufferToImageInfo,
    PrimaryAutoCommandBuffer,
};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::swapchain::{
    PresentMode, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainPresentInfo, acquire_next_image,
};
use vulkano::sync::GpuFuture;
use vulkano::sync::future::FenceSignalFuture;

trait FrameFence {
    fn wait_idle(self: Box<Self>);
}

impl<F: GpuFuture> FrameFence for FenceSignalFuture<F> {
    fn wait_idle(self: Box<Self>) {
        self.wait(None).unwrap();
    }
}

const FRAMES_IN_FLIGHT: usize = 2;

pub struct VulkanDisplay {
    window: Arc<dyn winit::window::Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,
    cursor: CursorState,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    image_format: Format,
    overlay: VulkanOverlay,
    frame_futures: RefCell<Vec<Option<Box<dyn FrameFence>>>>,
    current_frame: Cell<usize>,
}

impl VulkanDisplay {
    pub fn new(window: Arc<dyn winit::window::Window>, width: u32, height: u32) -> Self {
        let lib = VulkanLibrary::new().unwrap();
        let required_exts = Surface::required_extensions(&window).unwrap();

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

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let (swapchain, images, image_format) =
            Self::create_swapchain(&device, &surface, [width, height]);

        let overlay = VulkanOverlay::new(
            width,
            height,
            memory_allocator.clone(),
            device.clone(),
            image_format,
            FRAMES_IN_FLIGHT,
        );

        Self {
            window,
            device,
            queue,
            swapchain,
            images,
            cursor: CursorState::new(),
            memory_allocator,
            command_buffer_allocator,
            image_format,
            overlay,
            frame_futures: RefCell::new((0..FRAMES_IN_FLIGHT).map(|_| None).collect()),
            current_frame: Cell::new(0),
        }
    }

    fn create_swapchain(
        device: &Arc<Device>,
        surface: &Arc<Surface>,
        extent: [u32; 2],
    ) -> (Arc<Swapchain>, Vec<Arc<Image>>, Format) {
        let caps = device
            .physical_device()
            .surface_capabilities(surface, Default::default())
            .unwrap();

        // Aim for at least 2 images (double buffering), respecting both min and max bounds.
        let desired = caps.min_image_count.max(2);
        let image_count = match caps.max_image_count {
            Some(limit) => desired.min(limit),
            None => desired,
        };

        let formats = device
            .physical_device()
            .surface_formats(surface, Default::default())
            .unwrap();

        // Prefer R8G8B8A8_UNORM to match CPU buffer layout; fall back to first available.
        let (image_format, image_color_space) = formats
            .iter()
            .find(|(f, _)| *f == Format::R8G8B8A8_UNORM)
            .copied()
            .unwrap_or(formats[0]);

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: image_count,
                image_format,
                image_color_space,
                image_extent: extent,
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                present_mode: PresentMode::Fifo,
                ..Default::default()
            },
        )
        .unwrap();

        (swapchain, images, image_format)
    }

    pub fn shared_device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn shared_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    fn device_extensions() -> DeviceExtensions {
        DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        }
    }

    /// Creates a host-visible staging buffer sized for an RGBA image of `width` x `height`,
    /// initialised from `pixels`.
    fn create_rgba_staging_buffer(
        &self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Subbuffer<[u8]> {
        let staging: Subbuffer<[u8]> = Buffer::new_slice(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (width * height * 4) as u64,
        )
        .unwrap();
        staging.write().unwrap().copy_from_slice(pixels);
        staging
    }

    /// Builds a new primary one-time-submit command buffer for the graphics queue.
    fn begin_command_buffer(&self) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
        AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
    }

    /// Waits for the oldest in-flight frame to finish, freeing its slot for reuse.
    fn begin_frame(&self) {
        let idx = self.current_frame.get() % FRAMES_IN_FLIGHT;
        if let Some(fence) = self.frame_futures.borrow_mut()[idx].take() {
            fence.wait_idle();
        }
    }

    /// Submits the frame and stores its fence; the CPU returns immediately.
    fn end_frame(
        &self,
        acquire_future: SwapchainAcquireFuture,
        command_buffer: Arc<PrimaryAutoCommandBuffer>,
        image_index: u32,
    ) {
        let idx = self.current_frame.get() % FRAMES_IN_FLIGHT;
        let fence = acquire_future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush()
            .unwrap();
        self.frame_futures.borrow_mut()[idx] = Some(Box::new(fence));
        self.current_frame.set(self.current_frame.get() + 1);
    }
}

impl Display for VulkanDisplay {
    fn present_cpu_frame(&self, pixels: &[u8]) {
        let [width, height] = self.swapchain.image_extent();

        // Swap R/B channels if the swapchain uses BGRA layout (common on Windows).
        let swapped_storage;
        let data: &[u8] = match self.image_format {
            Format::B8G8R8A8_UNORM | Format::B8G8R8A8_SRGB => {
                swapped_storage = pixels
                    .chunks_exact(4)
                    .flat_map(|p| [p[2], p[1], p[0], p[3]])
                    .collect::<Vec<u8>>();
                &swapped_storage
            }
            _ => pixels,
        };

        self.begin_frame();
        let staging = self.create_rgba_staging_buffer(width, height, data);
        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(self.swapchain.clone(), None).unwrap();
        let target_image = self.images[image_index as usize].clone();

        let mut builder = self.begin_command_buffer();
        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(staging, target_image))
            .unwrap();

        self.end_frame(acquire_future, builder.build().unwrap(), image_index);
    }

    fn present_vk_frame(&self, image: &Arc<Image>, overlay: Option<&[u8]>) {
        let [width, height] = self.swapchain.image_extent();

        self.begin_frame();
        let frame_idx = self.current_frame.get() % FRAMES_IN_FLIGHT;
        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(self.swapchain.clone(), None).unwrap();
        let target_image = self.images[image_index as usize].clone();

        let mut builder = self.begin_command_buffer();

        if let Some(overlay_bytes) = overlay {
            let staging = self.create_rgba_staging_buffer(width, height, overlay_bytes);
            builder
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    staging,
                    self.overlay.overlay_image(frame_idx),
                ))
                .unwrap();
        }

        builder
            .blit_image(BlitImageInfo::images(image.clone(), target_image.clone()))
            .unwrap();

        if overlay.is_some() {
            self.overlay
                .record_overlay_pass(&mut builder, target_image, width, height, frame_idx);
        }

        self.end_frame(acquire_future, builder.build().unwrap(), image_index);
    }

    fn resize(&mut self, width: u32, height: u32) {
        for slot in self.frame_futures.get_mut() {
            if let Some(fence) = slot.take() {
                fence.wait_idle();
            }
        }
        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: [width, height],
                ..self.swapchain.create_info()
            })
            .unwrap();
        self.swapchain = new_swapchain;
        self.images = new_images;
        self.overlay = VulkanOverlay::new(
            width,
            height,
            self.memory_allocator.clone(),
            self.device.clone(),
            self.image_format,
            FRAMES_IN_FLIGHT,
        );
    }

    fn capture_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cursor.capture(&*self.window)
    }

    fn release_mouse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cursor.release(&*self.window)
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn is_cursor_grabbed(&self) -> bool {
        self.cursor.is_grabbed()
    }

    fn window(&self) -> Arc<dyn winit::window::Window> {
        self.window.clone()
    }
}
