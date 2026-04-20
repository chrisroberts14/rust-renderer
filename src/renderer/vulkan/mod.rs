mod device;
pub mod display;
mod overlay;
mod pipeline;
mod shaders;
mod shadow;

use bytemuck::{Pod, Zeroable};
use std::cell::RefCell;
use std::sync::Arc;
use thiserror::Error;
use vulkano::buffer::{
    AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
    RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::{ImageView, ImageViewCreateInfo, ImageViewType};
use vulkano::image::{
    Image, ImageAspects, ImageCreateFlags, ImageCreateInfo, ImageSubresourceRange, ImageType,
    ImageUsage,
};
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
use crate::maths::GpuMat4;
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

struct UploadedMesh<'a> {
    object: &'a Object,
    vertex_buffer: Subbuffer<[VulkanVertex]>,
    index_buffer: Subbuffer<[u32]>,
}

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
    model: GpuMat4,
    view: GpuMat4,
    proj: GpuMat4,
    normal_mat: GpuMat4,
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
    shadow_render_pass: Arc<RenderPass>,
    shadow_pipeline: Arc<GraphicsPipeline>,
    shadow_sampler: Arc<Sampler>,
    shadow_spot_image: Arc<Image>,
    shadow_point_image: Arc<Image>,
    last_colour_image: RefCell<Option<Arc<Image>>>,
}

impl VulkanRenderer {
    pub fn from_display(
        display: &crate::renderer::vulkan::display::VulkanDisplay,
    ) -> Result<Self, VulkanRendererError> {
        let device = display.shared_device();
        let queue = display.shared_queue();
        Self::from_device(device, queue)
    }

    pub fn new() -> Result<Self, VulkanRendererError> {
        let (device, queue) = get_device(DeviceExtensions::empty())?;
        Self::from_device(device, queue)
    }

    fn from_device(device: Arc<Device>, queue: Arc<Queue>) -> Result<Self, VulkanRendererError> {
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

        let shadow_render_pass = shadow::create_shadow_render_pass(device.clone());
        let shadow_pipeline =
            shadow::create_shadow_pipeline(device.clone(), shadow_render_pass.clone());
        let shadow_sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .unwrap();

        let shadow_spot_image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: [shadow::SHADOW_MAP_SIZE, shadow::SHADOW_MAP_SIZE, 1],
                array_layers: MAX_LIGHTS as u32,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        let shadow_point_image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: [shadow::SHADOW_MAP_SIZE, shadow::SHADOW_MAP_SIZE, 1],
                array_layers: MAX_LIGHTS as u32 * 6,
                flags: ImageCreateFlags::CUBE_COMPATIBLE,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap();

        Ok(VulkanRenderer {
            device,
            queue,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            render_pass,
            pipeline,
            shadow_render_pass,
            shadow_pipeline,
            shadow_sampler,
            shadow_spot_image,
            shadow_point_image,
            last_colour_image: RefCell::new(None),
        })
    }

    fn upload_objects<'a>(&self, objects: &'a [Object]) -> Vec<UploadedMesh<'a>> {
        objects
            .iter()
            .filter(|obj| !obj.mesh.faces.is_empty())
            .map(|obj| {
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

                UploadedMesh {
                    object: obj,
                    vertex_buffer,
                    index_buffer,
                }
            })
            .collect()
    }

    fn build_uniforms(&self, obj: &Object, camera: &Camera, ambient: f32) -> Subbuffer<VkUniforms> {
        let (model, normal_mat) = obj.transform.matrices();
        let proj = vk_perspective(camera.fov, camera.aspect_ratio, camera.near, camera.far);
        let data = VkUniforms {
            model: model.into(),
            view: camera.view_matrix().into(),
            proj: proj.into(),
            normal_mat: normal_mat.into(),
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

    fn build_shadow_block(&self, lights: &[Arc<dyn Light>]) -> Subbuffer<shadow::VkShadowBlock> {
        let mut block = shadow::VkShadowBlock {
            spot_light_space: [<GpuMat4 as bytemuck::Zeroable>::zeroed(); MAX_LIGHTS],
            point_far_plane: shadow::SHADOW_FAR,
            _pad: [0.0; 3],
        };
        for (i, light) in lights.iter().take(MAX_LIGHTS).enumerate() {
            if let Some(dir) = light.spot_direction() {
                let pos = light.position();
                let up = shadow::spot_up_vector(dir);
                let view = shadow::look_at(pos, pos + dir, up);
                let proj = vk_perspective(light.cone_angle() * 2.0, 1.0, 0.1, shadow::SHADOW_FAR);
                block.spot_light_space[i] = (proj * view).into();
            }
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

    fn render_shadow_pass(
        &self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        uploaded: &[UploadedMesh<'_>],
        lights: &[Arc<dyn Light>],
    ) -> shadow::ShadowMaps {
        let spot_image = &self.shadow_spot_image;
        let point_image = &self.shadow_point_image;

        let cube_proj = vk_perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, shadow::SHADOW_FAR);

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [
                shadow::SHADOW_MAP_SIZE as f32,
                shadow::SHADOW_MAP_SIZE as f32,
            ],
            depth_range: 0.0..=1.0,
        };

        // Spot shadow passes — only record a render pass for occupied light slots.
        for i in 0..MAX_LIGHTS {
            let Some((light_vp, light_pos)) = lights.get(i).and_then(|l| {
                l.spot_direction().map(|dir| {
                    let pos = l.position();
                    let up = shadow::spot_up_vector(dir);
                    let view = shadow::look_at(pos, pos + dir, up);
                    let proj = vk_perspective(l.cone_angle() * 2.0, 1.0, 0.1, shadow::SHADOW_FAR);
                    (proj * view, pos)
                })
            }) else {
                continue;
            };

            let layer_view = ImageView::new(
                spot_image.clone(),
                ImageViewCreateInfo {
                    view_type: ImageViewType::Dim2d,
                    subresource_range: ImageSubresourceRange {
                        aspects: ImageAspects::DEPTH,
                        mip_levels: 0..1,
                        array_layers: i as u32..i as u32 + 1,
                    },
                    ..ImageViewCreateInfo::from_image(spot_image)
                },
            )
            .unwrap();

            let fb = VkFramebuffer::new(
                self.shadow_render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![layer_view],
                    ..Default::default()
                },
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some(1.0f32.into())],
                        ..RenderPassBeginInfo::framebuffer(fb)
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    },
                )
                .unwrap();

            builder
                .set_viewport(0, [viewport.clone()].into_iter().collect())
                .unwrap()
                .bind_pipeline_graphics(self.shadow_pipeline.clone())
                .unwrap();

            for mesh in uploaded {
                let (model, _) = mesh.object.transform.matrices();
                let uniforms = shadow::VkShadowUniforms {
                    light_vp: light_vp.into(),
                    model: model.into(),
                    light_pos: [light_pos.x, light_pos.y, light_pos.z, 0.0],
                    shadow_far: shadow::SHADOW_FAR,
                    _pad: [0.0; 3],
                };
                let ubuf = Buffer::from_data(
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
                    uniforms,
                )
                .unwrap();
                let ds = DescriptorSet::new(
                    self.descriptor_set_allocator.clone(),
                    self.shadow_pipeline.layout().set_layouts()[0].clone(),
                    [WriteDescriptorSet::buffer(0, ubuf)],
                    [],
                )
                .unwrap();
                builder
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.shadow_pipeline.layout().clone(),
                        0,
                        ds,
                    )
                    .unwrap()
                    .bind_vertex_buffers(0, mesh.vertex_buffer.clone())
                    .unwrap()
                    .bind_index_buffer(mesh.index_buffer.clone())
                    .unwrap();
                unsafe {
                    builder
                        .draw_indexed(mesh.index_buffer.len() as u32, 1, 0, 0, 0)
                        .unwrap();
                }
            }

            builder.end_render_pass(SubpassEndInfo::default()).unwrap();
        }

        // Point shadow passes — 6 passes per occupied light slot.
        for i in 0..MAX_LIGHTS {
            let Some((face_views, light_pos)) = lights.get(i).and_then(|l| {
                if l.spot_direction().is_none() {
                    let pos = l.position();
                    Some((shadow::cube_face_views(pos), pos))
                } else {
                    None
                }
            }) else {
                continue;
            };

            for (face, _) in face_views.iter().enumerate() {
                let layer = (i * 6 + face) as u32;
                let light_vp = cube_proj * face_views[face];

                let layer_view = ImageView::new(
                    point_image.clone(),
                    ImageViewCreateInfo {
                        view_type: ImageViewType::Dim2d,
                        subresource_range: ImageSubresourceRange {
                            aspects: ImageAspects::DEPTH,
                            mip_levels: 0..1,
                            array_layers: layer..layer + 1,
                        },
                        ..ImageViewCreateInfo::from_image(point_image)
                    },
                )
                .unwrap();

                let fb = VkFramebuffer::new(
                    self.shadow_render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![layer_view],
                        ..Default::default()
                    },
                )
                .unwrap();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![Some(1.0f32.into())],
                            ..RenderPassBeginInfo::framebuffer(fb)
                        },
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .unwrap();

                builder
                    .set_viewport(0, [viewport.clone()].into_iter().collect())
                    .unwrap()
                    .bind_pipeline_graphics(self.shadow_pipeline.clone())
                    .unwrap();

                for mesh in uploaded {
                    let (model, _) = mesh.object.transform.matrices();
                    let uniforms = shadow::VkShadowUniforms {
                        light_vp: light_vp.into(),
                        model: model.into(),
                        light_pos: [light_pos.x, light_pos.y, light_pos.z, 0.0],
                        shadow_far: shadow::SHADOW_FAR,
                        _pad: [0.0; 3],
                    };
                    let ubuf = Buffer::from_data(
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
                        uniforms,
                    )
                    .unwrap();
                    let ds = DescriptorSet::new(
                        self.descriptor_set_allocator.clone(),
                        self.shadow_pipeline.layout().set_layouts()[0].clone(),
                        [WriteDescriptorSet::buffer(0, ubuf)],
                        [],
                    )
                    .unwrap();
                    builder
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            self.shadow_pipeline.layout().clone(),
                            0,
                            ds,
                        )
                        .unwrap()
                        .bind_vertex_buffers(0, mesh.vertex_buffer.clone())
                        .unwrap()
                        .bind_index_buffer(mesh.index_buffer.clone())
                        .unwrap();
                    unsafe {
                        builder
                            .draw_indexed(mesh.index_buffer.len() as u32, 1, 0, 0, 0)
                            .unwrap();
                    }
                }

                builder.end_render_pass(SubpassEndInfo::default()).unwrap();
            }
        }

        let spot_array_view = ImageView::new(
            spot_image.clone(),
            ImageViewCreateInfo {
                view_type: ImageViewType::Dim2dArray,
                subresource_range: ImageSubresourceRange {
                    aspects: ImageAspects::DEPTH,
                    mip_levels: 0..1,
                    array_layers: 0..MAX_LIGHTS as u32,
                },
                ..ImageViewCreateInfo::from_image(spot_image)
            },
        )
        .unwrap();

        let point_array_view = ImageView::new(
            point_image.clone(),
            ImageViewCreateInfo {
                view_type: ImageViewType::CubeArray,
                subresource_range: ImageSubresourceRange {
                    aspects: ImageAspects::DEPTH,
                    mip_levels: 0..1,
                    array_layers: 0..MAX_LIGHTS as u32 * 6,
                },
                ..ImageViewCreateInfo::from_image(point_image)
            },
        )
        .unwrap();

        shadow::ShadowMaps {
            spot_array_view,
            point_array_view,
            shadow_block_buf: self.build_shadow_block(lights),
        }
    }

    fn render_with_pipeline(
        &self,
        vk_pipeline: &Arc<GraphicsPipeline>,
        uploaded: &[UploadedMesh<'_>],
        camera: &Camera,
        framebuffer: &Framebuffer,
        ambient: f32,
        lights: &[Arc<dyn Light>],
    ) -> Vec<(&'static str, String)> {
        let width = framebuffer.width as u32;
        let height = framebuffer.height as u32;

        let colour_image = Image::new(
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

        let color_view = ImageView::new_default(colour_image.clone()).unwrap();
        let depth_view = ImageView::new_default(depth_image).unwrap();

        let vk_framebuffer = VkFramebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![color_view, depth_view],
                ..Default::default()
            },
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

        let shadow_maps = self.render_shadow_pass(&mut builder, uploaded, lights);

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                upload_buffer,
                colour_image.clone(),
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
        let triangle_count: usize = uploaded.iter().map(|u| u.object.mesh.faces.len()).sum();

        for mesh in uploaded {
            let index_count = mesh.index_buffer.len() as u32;
            let uniform_buf = self.build_uniforms(mesh.object, camera, ambient);
            let active_lights = if mesh.object.is_light {
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
                    WriteDescriptorSet::buffer(2, shadow_maps.shadow_block_buf.clone()),
                    WriteDescriptorSet::image_view_sampler(
                        3,
                        shadow_maps.spot_array_view.clone(),
                        self.shadow_sampler.clone(),
                    ),
                    WriteDescriptorSet::image_view_sampler(
                        4,
                        shadow_maps.point_array_view.clone(),
                        self.shadow_sampler.clone(),
                    ),
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
                .bind_vertex_buffers(0, mesh.vertex_buffer.clone())
                .unwrap()
                .bind_index_buffer(mesh.index_buffer.clone())
                .unwrap();

            // Safety: indices are [0,1,2, 3,4,5, ...] generated in sync with verts, so every index < verts.len().
            unsafe {
                builder.draw_indexed(index_count, 1, 0, 0, 0).unwrap();
            }
        }

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();

        let command_buffer = builder.build().unwrap();

        sync::now(self.device.clone())
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        *self.last_colour_image.borrow_mut() = Some(colour_image);

        vec![("Triangle Count", triangle_count.to_string())]
    }

    pub fn take_vk_image(&self) -> Option<Arc<Image>> {
        self.last_colour_image.borrow_mut().take()
    }
}

impl Default for VulkanRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create Vulkan renderer")
    }
}

pub fn into_active() -> super::ActiveRenderer {
    super::ActiveRenderer::Vulkan(Box::default())
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
        let uploaded = self.upload_objects(objects);
        self.render_with_pipeline(
            &self.pipeline.get_graphics_pipeline(PipelineType::Normal),
            &uploaded,
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
        let uploaded = self.upload_objects(objects);
        self.render_with_pipeline(
            &self.pipeline.get_graphics_pipeline(PipelineType::WireFrame),
            &uploaded,
            camera,
            framebuffer,
            1.0,
            &[],
        )
    }
}
