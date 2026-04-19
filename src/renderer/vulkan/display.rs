use crate::display::{CursorState, Display};
use crate::renderer::vulkan::device::get_device_for_surface;
use crate::renderer::vulkan::shaders;
use std::sync::Arc;
use vulkano::VulkanLibrary;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage, CopyBufferToImageInfo,
    PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
    SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{
    Framebuffer as VkFramebuffer, FramebufferCreateInfo, RenderPass, Subpass,
};
use vulkano::swapchain::{
    PresentMode, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainPresentInfo, acquire_next_image,
};
use vulkano::sync::GpuFuture;

pub struct VulkanDisplay {
    #[allow(dead_code)]
    window: Arc<dyn winit::window::Window>,
    #[allow(dead_code)]
    instance: Arc<Instance>,
    #[allow(dead_code)]
    surface: Arc<Surface>,
    #[allow(dead_code)]
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<Image>>,
    cursor: CursorState,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    image_format: Format,
    overlay_image: Arc<Image>,
    overlay_render_pass: Arc<RenderPass>,
    overlay_pipeline: Arc<GraphicsPipeline>,
    overlay_sampler: Arc<Sampler>,
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
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let (swapchain, images, image_format) =
            Self::create_swapchain(&device, &surface, [width, height]);

        let overlay_image = Self::create_overlay_image(&memory_allocator, width, height);
        let overlay_render_pass = Self::create_overlay_render_pass(&device, image_format);
        let overlay_pipeline = Self::create_overlay_pipeline(&device, &overlay_render_pass);
        let overlay_sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        Self {
            window,
            instance,
            surface,
            device,
            queue,
            swapchain,
            images,
            cursor: CursorState::new(),
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            image_format,
            overlay_image,
            overlay_render_pass,
            overlay_pipeline,
            overlay_sampler,
        }
    }

    fn create_overlay_image(
        memory_allocator: &Arc<StandardMemoryAllocator>,
        width: u32,
        height: u32,
    ) -> Arc<Image> {
        Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn create_overlay_render_pass(device: &Arc<Device>, format: Format) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: format,
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap()
    }

    fn create_overlay_pipeline(
        device: &Arc<Device>,
        render_pass: &Arc<RenderPass>,
    ) -> Arc<GraphicsPipeline> {
        let vs = shaders::overlay_vs::load(device.clone())
            .unwrap()
            .entry_point("main")
            .unwrap();
        let fs = shaders::overlay_fs::load(device.clone())
            .unwrap()
            .entry_point("main")
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let blend = AttachmentBlend {
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
        };
        let color_blend_state = ColorBlendState::with_attachment_states(
            1,
            ColorBlendAttachmentState {
                blend: Some(blend),
                ..Default::default()
            },
        );

        GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(VertexInputState::default()),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(color_blend_state),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
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

    /// Submits `command_buffer` after `acquire_future` and presents `image_index` to the
    /// swapchain, blocking until the GPU signals completion.
    fn submit_and_present(
        &self,
        acquire_future: SwapchainAcquireFuture,
        command_buffer: Arc<PrimaryAutoCommandBuffer>,
        image_index: u32,
    ) {
        acquire_future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();
    }

    /// Records the overlay compositing pass (fullscreen-triangle alpha blend) targeting the
    /// given swapchain image.
    fn record_overlay_pass(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        swapchain_image: Arc<Image>,
        width: u32,
        height: u32,
    ) {
        let swapchain_view = ImageView::new_default(swapchain_image).unwrap();
        let framebuffer = VkFramebuffer::new(
            self.overlay_render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![swapchain_view],
                ..Default::default()
            },
        )
        .unwrap();

        let overlay_view = ImageView::new_default(self.overlay_image.clone()).unwrap();
        let descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            self.overlay_pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                overlay_view,
                self.overlay_sampler.clone(),
            )],
            [],
        )
        .unwrap();

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [width as f32, height as f32],
            depth_range: 0.0..=1.0,
        };

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![None],
                    ..RenderPassBeginInfo::framebuffer(framebuffer)
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap()
            .set_viewport(0, [viewport].into_iter().collect())
            .unwrap()
            .bind_pipeline_graphics(self.overlay_pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.overlay_pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .unwrap();

        // Safety: no vertex buffer; overlay_vs generates the fullscreen triangle from
        // gl_VertexIndex with no vertex input attributes.
        unsafe {
            builder.draw(3, 1, 0, 0).unwrap();
        }

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();
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

        let staging = self.create_rgba_staging_buffer(width, height, data);

        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(self.swapchain.clone(), None).unwrap();
        let target_image = self.images[image_index as usize].clone();

        let mut builder = self.begin_command_buffer();
        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(staging, target_image))
            .unwrap();

        let command_buffer = builder.build().unwrap();
        self.submit_and_present(acquire_future, command_buffer, image_index);
    }

    fn present_vk_frame(&self, image: &Arc<Image>, overlay: Option<&[u8]>) {
        let [width, height] = self.swapchain.image_extent();
        let (image_index, _suboptimal, acquire_future) =
            acquire_next_image(self.swapchain.clone(), None).unwrap();
        let target_image = self.images[image_index as usize].clone();

        let mut builder = self.begin_command_buffer();

        if let Some(overlay_bytes) = overlay {
            let staging = self.create_rgba_staging_buffer(width, height, overlay_bytes);
            builder
                .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                    staging,
                    self.overlay_image.clone(),
                ))
                .unwrap();
        }

        builder
            .blit_image(BlitImageInfo::images(image.clone(), target_image.clone()))
            .unwrap();

        if overlay.is_some() {
            self.record_overlay_pass(&mut builder, target_image, width, height);
        }

        let command_buffer = builder.build().unwrap();
        self.submit_and_present(acquire_future, command_buffer, image_index);
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
        self.overlay_image = Self::create_overlay_image(&self.memory_allocator, width, height);
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
