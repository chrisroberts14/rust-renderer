use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use vulkano::buffer::Subbuffer;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;

use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{RenderPass, Subpass};

use crate::maths::mat4::Mat4;
use crate::maths::vec3::Vec3;

pub(super) const SHADOW_MAP_SIZE: u32 = 1024;
pub(super) const SHADOW_FAR: f32 = 100.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct VkShadowBlock {
    pub spot_light_space: [[[f32; 4]; 4]; 8],
    pub point_far_plane: f32,
    pub _pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct VkShadowUniforms {
    pub light_vp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_pos: [f32; 4],
    pub shadow_far: f32,
    pub _pad: [f32; 3],
}

pub(super) struct ShadowMaps {
    pub spot_array_view: Arc<ImageView>,
    pub point_array_view: Arc<ImageView>,
    pub shadow_block_buf: Subbuffer<VkShadowBlock>,
}

pub(super) fn create_shadow_render_pass(device: Arc<Device>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            depth: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
        },
        pass: {
            color: [],
            depth_stencil: {depth},
        },
    )
    .unwrap()
}

pub(super) fn create_shadow_pipeline(
    device: Arc<Device>,
    render_pass: Arc<RenderPass>,
) -> Arc<GraphicsPipeline> {
    let vs = super::shaders::shadow_vs::load(device.clone())
        .unwrap()
        .entry_point("main")
        .unwrap();
    let fs = super::shaders::shadow_fs::load(device.clone())
        .unwrap()
        .entry_point("main")
        .unwrap();

    let vertex_input_state = super::VulkanVertex::per_vertex().definition(&vs).unwrap();
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
    let subpass = Subpass::from(render_pass, 0).unwrap();

    GraphicsPipeline::new(
        device,
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState::default()),
            rasterization_state: Some(RasterizationState {
                cull_mode: CullMode::Front,
                ..Default::default()
            }),
            multisample_state: Some(MultisampleState::default()),
            depth_stencil_state: Some(DepthStencilState {
                depth: Some(DepthState::simple()),
                ..Default::default()
            }),

            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .unwrap()
}

pub(super) fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = (target - eye).normalise();
    let r = f.cross(up).normalise();
    let u = r.cross(f);
    Mat4 {
        m: [
            [r.x, r.y, r.z, -r.dot(eye)],
            [u.x, u.y, u.z, -u.dot(eye)],
            [-f.x, -f.y, -f.z, f.dot(eye)],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

// View matrices for each of the 6 cube faces, in Vulkan face order (+X,-X,+Y,-Y,+Z,-Z).
pub(super) fn cube_face_views(pos: Vec3) -> [Mat4; 6] {
    [
        look_at(
            pos,
            pos + Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ),
        look_at(
            pos,
            pos + Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ),
        look_at(
            pos,
            pos + Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ),
        look_at(
            pos,
            pos + Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        ),
        look_at(
            pos,
            pos + Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, -1.0, 0.0),
        ),
        look_at(
            pos,
            pos + Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, -1.0, 0.0),
        ),
    ]
}

pub(super) fn spot_up_vector(dir: Vec3) -> Vec3 {
    let world_up = Vec3::new(0.0, 1.0, 0.0);
    if dir.dot(world_up).abs() > 0.99 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        world_up
    }
}
