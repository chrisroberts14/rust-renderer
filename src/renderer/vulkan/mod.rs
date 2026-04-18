mod device;
mod pipeline;
mod shaders;

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
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipeline;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer as VkFramebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::sync::{self, GpuFuture};
use vulkano::{LoadingError, Validated, VulkanError};

use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::maths::mat4::Mat4;
use crate::maths::vec3::Vec3;
use crate::renderer::Renderer;
use crate::renderer::vulkan::device::get_device;
use crate::renderer::vulkan::pipeline::{Pipeline as VulkanPipeline, PipelineType};
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;

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
    pipeline: VulkanPipeline,
}

impl VulkanRenderer {
    pub fn new() -> Result<Self, VulkanRendererError> {
        let (device, queue) = get_device()?;

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

        let pipeline = VulkanPipeline::new(device.clone(), render_pass.clone())?;

        Ok(VulkanRenderer {
            device,
            queue,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            render_pass,
            pipeline,
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

    fn draw_to_framebuffer(staging_buffer: Subbuffer<[u8]>, framebuffer: &Framebuffer) {
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
    }

    fn render_with_pipeline(
        &self,
        vk_pipeline: &Arc<GraphicsPipeline>,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
        ambient: f32,
        lights: &[Arc<dyn Light>],
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
            .bind_pipeline_graphics(vk_pipeline.clone())
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
                vk_pipeline.layout().set_layouts()[0].clone(),
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
                    vk_pipeline.layout().clone(),
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

        Self::draw_to_framebuffer(staging_buffer, framebuffer);

        vec![("Triangle Count", triangle_count.to_string())]
    }
}

pub fn into_active() -> super::ActiveRenderer {
    super::ActiveRenderer::Vulkan(Box::new(
        VulkanRenderer::new().expect("Failed to create Vulkan renderer"),
    ))
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
        self.render_with_pipeline(
            &self.pipeline.get_graphics_pipeline(PipelineType::Normal),
            objects,
            camera,
            framebuffer,
            ambient,
            lights,
        )
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        self.render_with_pipeline(
            &self.pipeline.get_graphics_pipeline(PipelineType::WireFrame),
            objects,
            camera,
            framebuffer,
            1.0,
            &[],
        )
    }
}
