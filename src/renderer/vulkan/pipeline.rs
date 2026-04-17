//! Thin wrapper around Vulkan pipelines.
//! Since Vulkan pipelines have to be created ahead of time we must define multiple pipelines
//! if we want to change settings.

use crate::renderer::vulkan::{VulkanRendererError, VulkanVertex, shaders};
use std::sync::Arc;
use vulkano::device::Device;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{CullMode, PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition, VertexInputState};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{RenderPass, Subpass};

/// The selection of pipelines
/// Each new pipeline should be registered here
pub(crate) enum PipelineType {
    Normal,
    WireFrame,
}

/// Wrapper around Vulkano `GraphicsPipeline` which can contain multiple pipelines
pub(crate) struct Pipeline {
    wireframe_pipeline: Arc<GraphicsPipeline>,
    normal_pipeline: Arc<GraphicsPipeline>,
}

impl Pipeline {
    /// Get a given pipeline based on a registered type
    pub fn get_graphics_pipeline(&self, pipeline_type: PipelineType) -> Arc<GraphicsPipeline> {
        match pipeline_type {
            PipelineType::Normal => self.normal_pipeline.clone(),
            PipelineType::WireFrame => self.wireframe_pipeline.clone(),
        }
    }

    pub fn new(
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
    ) -> Result<Self, VulkanRendererError> {
        let vs = shaders::vs::load(device.clone())?
            .entry_point("main")
            .unwrap();
        let fs = shaders::fs::load(device.clone())?
            .entry_point("main")
            .unwrap();

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
        )?;

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        // Create the two pipelines
        let wireframe_pipeline = Self::create_pipeline(
            device.clone(),
            stages.clone(),
            vertex_input_state.clone(),
            subpass.clone(),
            layout.clone(),
            PipelineType::Normal,
        )
        .expect("Failed to create wireframe pipeline");
        let normal_pipeline = Self::create_pipeline(
            device.clone(),
            stages.clone(),
            vertex_input_state.clone(),
            subpass.clone(),
            layout.clone(),
            PipelineType::Normal,
        )
        .expect("Failed to create normal pipeline");
        Ok(Self {
            wireframe_pipeline,
            normal_pipeline,
        })
    }

    fn create_pipeline(
        device: Arc<Device>,
        stages: [PipelineShaderStageCreateInfo; 2],
        vertex_input_state: VertexInputState,
        subpass: Subpass,
        layout: Arc<PipelineLayout>,
        pipeline_type: PipelineType,
    ) -> Result<Arc<GraphicsPipeline>, VulkanRendererError> {
        let rasterization_state = match pipeline_type {
            PipelineType::Normal => RasterizationState {
                polygon_mode: PolygonMode::Fill,
                cull_mode: CullMode::Front,
                ..Default::default()
            },
            PipelineType::WireFrame => RasterizationState {
                polygon_mode: PolygonMode::Line,
                cull_mode: CullMode::Front,
                ..Default::default()
            },
        };

        Ok(GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(rasterization_state),
                multisample_state: Some(MultisampleState::default()),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )?)
    }
}
