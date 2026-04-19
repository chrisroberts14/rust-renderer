use crate::renderer::vulkan::shaders;
use std::sync::Arc;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassContents, SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
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

pub(crate) struct VulkanOverlay {
    overlay_image: Arc<Image>,
    overlay_render_pass: Arc<RenderPass>,
    overlay_pipeline: Arc<GraphicsPipeline>,
    overlay_sampler: Arc<Sampler>,

    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

impl VulkanOverlay {
    pub fn new(
        width: u32,
        height: u32,
        memory_allocator: Arc<StandardMemoryAllocator>,
        device: Arc<Device>,
        format: Format,
    ) -> Self {
        let overlay_image = Image::new(
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
        .unwrap();
        let overlay_render_pass = vulkano::single_pass_renderpass!(
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
        .unwrap();

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

        let subpass = Subpass::from(overlay_render_pass.clone(), 0).unwrap();

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

        let overlay_pipeline = GraphicsPipeline::new(
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
        .unwrap();
        let overlay_sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Self {
            overlay_image,
            overlay_pipeline,
            overlay_render_pass,
            overlay_sampler,
            descriptor_set_allocator,
        }
    }

    /// Records the overlay compositing pass (fullscreen-triangle alpha blend) targeting the
    /// given swapchain image.
    pub(crate) fn record_overlay_pass(
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

    pub(crate) fn overlay_image(&self) -> Arc<Image> {
        self.overlay_image.clone()
    }
}
