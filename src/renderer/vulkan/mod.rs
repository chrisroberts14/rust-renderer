use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use thiserror::Error;
use vulkano::buffer::{
    AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, CopyImageToBufferInfo,
    RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{CullMode, PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{
    Framebuffer as VkFramebuffer, FramebufferCreateInfo, RenderPass, Subpass,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{LoadingError, Validated, VulkanError, VulkanLibrary};

use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::maths::mat4::Mat4;
use crate::maths::vec3::Vec3;
use crate::renderer::Renderer;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;
            layout(location = 2) in vec4 color;

            layout(location = 0) out vec3 frag_world_pos;
            layout(location = 1) out vec3 frag_normal;
            layout(location = 2) out vec4 frag_color;

            layout(set = 0, binding = 0) uniform Uniforms {
                mat4 model;
                mat4 view;
                mat4 proj;
                mat4 normal_mat;
                vec4 cam_pos;
                float ambient;
            };

            void main() {
                vec4 world_pos = model * vec4(position, 1.0);
                frag_world_pos = world_pos.xyz;
                frag_normal = normalize((normal_mat * vec4(normal, 0.0)).xyz);
                frag_color = color;
                gl_Position = proj * view * world_pos;
                // Vulkan NDC has Y+ pointing down; flip to match scene conventions.
                gl_Position.y = -gl_Position.y;
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec3 frag_world_pos;
            layout(location = 1) in vec3 frag_normal;
            layout(location = 2) in vec4 frag_color;

            layout(location = 0) out vec4 out_color;

            layout(set = 0, binding = 0) uniform Uniforms {
                mat4 model;
                mat4 view;
                mat4 proj;
                mat4 normal_mat;
                vec4 cam_pos;
                float ambient;
            };

            struct Light {
                vec4 position;   // xyz = world pos, w = intensity
                vec4 color;      // xyz = rgb, w = unused
                vec4 direction;  // xyz = spot dir, w = cone_angle (0 = point)
                vec4 falloff;    // x = falloff_angle
            };

            layout(set = 0, binding = 1) uniform LightBlock {
                Light lights[8];
                uint light_count;
            };

            void main() {
                vec3 n = normalize(frag_normal);
                vec4 base = frag_color;

                if (light_count == 0u) {
                    out_color = base;
                    return;
                }

                vec3 view_dir = normalize(cam_pos.xyz - frag_world_pos);
                vec3 diffuse  = vec3(0.0);
                vec3 specular = vec3(0.0);

                for (uint i = 0u; i < light_count; i++) {
                    vec3  lpos        = lights[i].position.xyz;
                    float intensity   = lights[i].position.w;
                    vec3  diff_vec    = lpos - frag_world_pos;
                    float dist_sq     = dot(diff_vec, diff_vec);
                    float dist_atten  = intensity / (1.0 + dist_sq);

                    float cone_atten  = 1.0;
                    float cone_angle  = lights[i].direction.w;
                    if (cone_angle > 0.0) {
                        vec3  spot_dir      = lights[i].direction.xyz;
                        float falloff_angle = lights[i].falloff.x;
                        vec3  to_point      = normalize(frag_world_pos - lpos);
                        float angle         = acos(clamp(dot(spot_dir, to_point), -1.0, 1.0));
                        if (angle > cone_angle) {
                            cone_atten = 0.0;
                        } else {
                            float inner_angle = cone_angle - falloff_angle;
                            if (angle > inner_angle) {
                                float t = (angle - inner_angle) / falloff_angle;
                                cone_atten = 1.0 - t * t * (3.0 - 2.0 * t);
                            }
                        }
                    }

                    vec3  lcol  = lights[i].color.xyz * (dist_atten * cone_atten);
                    vec3  ldir  = normalize(diff_vec);
                    float ndotl = max(dot(n, ldir), 0.0);
                    diffuse    += ndotl * lcol;
                    if (ndotl > 0.0) {
                        vec3 refl = reflect(-ldir, n);
                        specular += pow(max(dot(refl, view_dir), 0.0), 32.0) * lcol;
                    }
                }

                float inv_amb = 1.0 - ambient;
                vec3  lit = clamp(vec3(ambient) + inv_amb * diffuse + specular, 0.0, 1.0);
                out_color = vec4(base.rgb * lit, base.a);
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

const MAX_LIGHTS: usize = 8;

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct VulkanVertex {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    normal: [f32; 3],
    #[format(R32G32B32A32_SFLOAT)]
    color: [f32; 4],
}

// Mat4 is row-major; GLSL mat4 is column-major — transpose before upload.
fn mat4_to_cols(m: Mat4) -> [[f32; 4]; 4] {
    m.transpose().m
}

fn vk_perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov * 0.5).tan();
    let nf = 1.0 / (near - far);
    Mat4 {
        m: [
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far * nf, far * near * nf],
            [0.0, 0.0, -1.0, 0.0],
        ],
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VkUniforms {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    normal_mat: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    ambient: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VkLight {
    position: [f32; 4],
    color: [f32; 4],
    direction: [f32; 4],
    falloff: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VkLightBlock {
    lights: [VkLight; MAX_LIGHTS],
    light_count: u32,
    _pad: [u32; 3],
}

pub struct VulkanRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    render_pass: Arc<RenderPass>,

    // Notably with Vulkan we have to create pipelines ahead of time so
    // we have to have different pipelines for wireframe and normal
    pipeline: Arc<GraphicsPipeline>,
    wireframe_pipeline: Arc<GraphicsPipeline>,
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
                enabled_features: DeviceFeatures {
                    fill_mode_non_solid: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: Format::R8G8B8A8_UNORM,
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                },
                depth: {
                    format: Format::D32_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {depth},
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
        )?;

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.clone().into_iter().collect(),
                vertex_input_state: Some(vertex_input_state.clone()),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState {
                    polygon_mode: PolygonMode::Fill,
                    cull_mode: CullMode::Front,
                    ..Default::default()
                }),
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
                subpass: Some(subpass.clone().into()),
                ..GraphicsPipelineCreateInfo::layout(layout.clone())
            },
        )?;

        let wireframe_pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState::default()),
                rasterization_state: Some(RasterizationState {
                    polygon_mode: PolygonMode::Line,
                    cull_mode: CullMode::Front,
                    ..Default::default()
                }),
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
        )?;

        Ok(VulkanRenderer {
            device,
            queue: queues.next().unwrap(),
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            render_pass,
            pipeline,
            wireframe_pipeline,
        })
    }

    fn upload_object(&self, obj: &Object) -> (Subbuffer<[VulkanVertex]>, Subbuffer<[u32]>) {
        let mut verts: Vec<VulkanVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for &(i0, i1, i2) in obj.mesh.faces.iter() {
            let base = verts.len() as u32;
            for vi in [i0, i1, i2] {
                let pos = obj.mesh.vertices[vi];
                let nor = obj.mesh.normals[vi];
                let color = match &obj.material {
                    Material::Color([r, g, b, a]) => [
                        *r as f32 / 255.0,
                        *g as f32 / 255.0,
                        *b as f32 / 255.0,
                        *a as f32 / 255.0,
                    ],
                    Material::Texture(_) => [1.0, 1.0, 1.0, 1.0],
                };
                verts.push(VulkanVertex {
                    position: [pos.x, pos.y, pos.z],
                    normal: [nor.x, nor.y, nor.z],
                    color,
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2]);
        }

        let vertex_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            verts,
        )
        .unwrap();

        let index_buffer = Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices,
        )
        .unwrap();

        (vertex_buffer, index_buffer)
    }

    fn build_uniforms(&self, obj: &Object, camera: &Camera, ambient: f32) -> Subbuffer<VkUniforms> {
        let (model, normal_mat) = obj.transform.matrices();
        let proj = vk_perspective(camera.fov, camera.aspect_ratio, camera.near, camera.far);
        let data = VkUniforms {
            model: mat4_to_cols(model),
            view: mat4_to_cols(camera.view_matrix()),
            proj: mat4_to_cols(proj),
            normal_mat: mat4_to_cols(normal_mat),
            cam_pos: [camera.position.x, camera.position.y, camera.position.z, 0.0],
            ambient,
            _pad: [0.0; 3],
        };
        Buffer::from_data(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )
        .unwrap()
    }

    fn build_light_block(&self, lights: &[Arc<dyn Light>]) -> Subbuffer<VkLightBlock> {
        let mut block = VkLightBlock {
            lights: [VkLight {
                position: [0.0; 4],
                color: [0.0; 4],
                direction: [0.0; 4],
                falloff: [0.0; 4],
            }; MAX_LIGHTS],
            light_count: lights.len().min(MAX_LIGHTS) as u32,
            _pad: [0; 3],
        };
        for (i, light) in lights.iter().take(MAX_LIGHTS).enumerate() {
            let p = light.position();
            let c = light.colour();
            let intensity = light.intensity();
            let (dir, cone, falloff) = match light.spot_direction() {
                Some(d) => (d, light.cone_angle(), light.falloff_angle()),
                None => (Vec3::ZERO, 0.0_f32, 0.0_f32),
            };
            block.lights[i] = VkLight {
                position: [p.x, p.y, p.z, intensity],
                color: [c[0], c[1], c[2], 1.0],
                direction: [dir.x, dir.y, dir.z, cone],
                falloff: [falloff, 0.0, 0.0, 0.0],
            };
        }
        Buffer::from_data(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            block,
        )
        .unwrap()
    }
}

impl Renderer for VulkanRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        let width = framebuffer.width as u32;
        let height = framebuffer.height as u32;

        let color_image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_SRC
                    | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let depth_image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: [width, height, 1],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let color_view = ImageView::new_default(color_image.clone()).unwrap();
        let depth_view = ImageView::new_default(depth_image).unwrap();

        let vk_framebuffer = VkFramebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![color_view, depth_view],
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

        // Seed color image from CPU framebuffer so the render pass composites on top (e.g. skybox).
        let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
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
        upload_buffer
            .write()
            .unwrap()
            .copy_from_slice(framebuffer.as_bytes());

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                upload_buffer,
                color_image.clone(),
            ))
            .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        None,                // color: LoadOp::Load, seeded above
                        Some(1.0f32.into()), // depth clear
                    ],
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
            .unwrap();

        let light_block = self.build_light_block(lights);
        let empty_light_block = self.build_light_block(&[]);
        let triangle_count: usize = objects.iter().map(|o| o.mesh.faces.len()).sum();

        for obj in objects {
            if obj.mesh.faces.is_empty() {
                continue;
            }
            let (vbuf, ibuf) = self.upload_object(obj);
            let index_count = ibuf.len() as u32;
            let uniform_buf = self.build_uniforms(obj, camera, ambient);
            let active_lights = if obj.is_light {
                &empty_light_block
            } else {
                &light_block
            };

            let descriptor_set = DescriptorSet::new(
                self.descriptor_set_allocator.clone(),
                self.pipeline.layout().set_layouts()[0].clone(),
                [
                    WriteDescriptorSet::buffer(0, uniform_buf),
                    WriteDescriptorSet::buffer(1, active_lights.clone()),
                ],
                [],
            )
            .unwrap();

            builder
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .unwrap()
                .bind_vertex_buffers(0, vbuf)
                .unwrap()
                .bind_index_buffer(ibuf)
                .unwrap();

            // Safety: indices are [0,1,2, 3,4,5, ...] generated in sync with verts, so every index < verts.len().
            unsafe {
                builder.draw_indexed(index_count, 1, 0, 0, 0).unwrap();
            }
        }

        builder
            .end_render_pass(SubpassEndInfo::default())
            .unwrap()
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                color_image,
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

        vec![("Triangle Count", triangle_count.to_string())]
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        let width = framebuffer.width as u32;
        let height = framebuffer.height as u32;

        let color_image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [width, height, 1],
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_SRC
                    | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let depth_image = Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: [width, height, 1],
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let color_view = ImageView::new_default(color_image.clone()).unwrap();
        let depth_view = ImageView::new_default(depth_image).unwrap();

        let vk_framebuffer = VkFramebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![color_view, depth_view],
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

        // Seed color image from CPU framebuffer so the render pass composites on top (e.g. skybox).
        let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
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
        upload_buffer
            .write()
            .unwrap()
            .copy_from_slice(framebuffer.as_bytes());

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                upload_buffer,
                color_image.clone(),
            ))
            .unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        None,                // color: LoadOp::Load, seeded above
                        Some(1.0f32.into()), // depth clear
                    ],
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
            .bind_pipeline_graphics(self.wireframe_pipeline.clone())
            .unwrap();

        let triangle_count: usize = objects.iter().map(|o| o.mesh.faces.len()).sum();
        let empty_light_block = self.build_light_block(&[]);

        for obj in objects {
            if obj.mesh.faces.is_empty() {
                continue;
            }
            let (vbuf, ibuf) = self.upload_object(obj);
            let index_count = ibuf.len() as u32;
            let uniform_buf = self.build_uniforms(obj, camera, 1.0);

            let descriptor_set = DescriptorSet::new(
                self.descriptor_set_allocator.clone(),
                self.wireframe_pipeline.layout().set_layouts()[0].clone(),
                [
                    WriteDescriptorSet::buffer(0, uniform_buf),
                    WriteDescriptorSet::buffer(1, empty_light_block.clone()),
                ],
                [],
            )
            .unwrap();

            builder
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.wireframe_pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .unwrap()
                .bind_vertex_buffers(0, vbuf)
                .unwrap()
                .bind_index_buffer(ibuf)
                .unwrap();

            // Safety: indices are [0,1,2, 3,4,5, ...] generated in sync with verts, so every index < verts.len().
            unsafe {
                builder.draw_indexed(index_count, 1, 0, 0, 0).unwrap();
            }
        }

        builder
            .end_render_pass(SubpassEndInfo::default())
            .unwrap()
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                color_image,
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

        vec![("Triangle Count", triangle_count.to_string())]
    }
}
