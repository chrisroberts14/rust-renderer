use std::sync::Arc;
use thiserror::Error;
use vulkano::buffer::{
    AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyImageToBufferInfo, RenderPassBeginInfo,
    SubpassBeginInfo, SubpassContents, SubpassEndInfo,
};
use vulkano::device::{Device, DeviceCreateInfo, Queue, QueueCreateInfo, QueueFlags};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{
    Framebuffer as VkFramebuffer, FramebufferCreateInfo, RenderPass, Subpass,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{LoadingError, Validated, VulkanError, VulkanLibrary};

use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::renderer::Renderer;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450
            layout(location = 0) in vec2 position;
            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450
            layout(location = 0) out vec4 f_color;
            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

#[derive(Error, Debug)]
pub enum VulkanRendererError {
    #[error("Failed to load Vulkan library: {0}")]
    Library(#[from] LoadingError),

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] Validated<VulkanError>),

    #[error("Failed to enumerate physical devices: {0}")]
    PhysicalDevices(#[from] VulkanError),

    #[error("No physical devices found")]
    NoPhysicalDevice,

    #[error("Could not find graphical queue family")]
    NoGraphicalQueueFamily,

    #[error("Failed to allocate buffer: {0}")]
    BufferError(#[from] Validated<AllocateBufferError>),
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct VulkanVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

pub struct VulkanRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    vertex_buffer: Subbuffer<[VulkanVertex]>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
}

impl VulkanRenderer {
    pub fn new() -> Result<Self, VulkanRendererError> {
        let library = VulkanLibrary::new()?;
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                ..Default::default()
            },
        )?;

        let physical_device = instance
            .enumerate_physical_devices()?
            .next()
            .ok_or(VulkanRendererError::NoPhysicalDevice)?;

        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .position(|q| q.queue_flags.contains(QueueFlags::GRAPHICS))
            .ok_or(VulkanRendererError::NoGraphicalQueueFamily)?
            as u32;

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )?;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let vertex_buffer: Subbuffer<[VulkanVertex]> = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![
                VulkanVertex {
                    position: [-0.5, -0.5],
                },
                VulkanVertex {
                    position: [0.0, 0.5],
                },
                VulkanVertex {
                    position: [0.5, -0.25],
                },
            ],
        )?;

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: Format::R8G8B8A8_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )?;

        let vs = vs::load(device.clone())?.entry_point("main").unwrap();
        let fs = fs::load(device.clone())?.entry_point("main").unwrap();

        let vertex_input_state = VulkanVertex::per_vertex().definition(&vs).unwrap();

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

        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )?;

        Ok(VulkanRenderer {
            device,
            queue: queues.next().unwrap(),
            memory_allocator,
            command_buffer_allocator,
            vertex_buffer,
            render_pass,
            pipeline,
        })
    }
}

impl Renderer for VulkanRenderer {
    fn render_objects(
        &self,
        _objects: &[Object],
        _camera: &Camera,
        _lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        _ambient: f32,
    ) -> Vec<(&'static str, String)> {
        let width = framebuffer.width as u32;
        let height = framebuffer.height as u32;

        let image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let image_view = ImageView::new_default(image.clone()).unwrap();

        let vk_framebuffer = VkFramebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![image_view],
                ..Default::default()
            },
        )
        .unwrap();

        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            (width * height * 4) as u64,
        )
        .unwrap();

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(vk_framebuffer)
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap()
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0, 0.0],
                    extent: [width as f32, height as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .unwrap();

        // Safety: vertex buffer contains exactly 3 vertices matching vertex_count.
        unsafe {
            builder.draw(3, 1, 0, 0).unwrap();
        }

        builder
            .end_render_pass(SubpassEndInfo::default())
            .unwrap()
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                image,
                staging_buffer.clone(),
            ))
            .unwrap();

        let command_buffer = builder.build().unwrap();

        sync::now(self.device.clone())
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        let buffer_content = staging_buffer.read().unwrap();
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let idx = (y * framebuffer.width + x) * 4;
                framebuffer.set_pixel(
                    x,
                    y,
                    [
                        buffer_content[idx],
                        buffer_content[idx + 1],
                        buffer_content[idx + 2],
                        buffer_content[idx + 3],
                    ],
                );
            }
        }

        vec![("triangles", "1".to_string())]
    }

    fn render_wireframe(
        &self,
        _objects: &[Object],
        _camera: &Camera,
        _framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        VulkanRenderer::new().unwrap();
    }
}
