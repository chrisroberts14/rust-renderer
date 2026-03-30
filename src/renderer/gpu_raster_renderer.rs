use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::renderer::{RenderStats, RendererChoice};
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// A renderer that runs on a GPU

const MAX_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuUniforms {
    model: [[f32; 4]; 4],      // 64 bytes
    view: [[f32; 4]; 4],       // 64 bytes
    proj: [[f32; 4]; 4],       // 64 bytes
    normal_mat: [[f32; 4]; 4], // 64 bytes
    cam_pos: [f32; 4],         // 16 bytes
    ambient: f32,              //  4 bytes
    _pad: [f32; 3],            // 12 bytes — pads struct to 288, matching WGSL alignment
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLight {
    position:  [f32; 4],  // xyz = world pos, w = intensity
    color:     [f32; 4],  // xyz = rgb, w = unused
    direction: [f32; 4],  // xyz = spot direction, w = cone_angle (0 = point light)
    falloff:   [f32; 4],  // x = falloff_angle, yzw = padding
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLightBlock {
    lights: [GpuLight; MAX_LIGHTS],
    light_count: u32,
    _pad: [u32; 3],
}

pub struct GpuRasterRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    // RefCell needed because render_objects takes &self but we lazily initialise/resize this
    colour_texture: RefCell<Option<GpuFramebuffer>>,
}

struct GpuFramebuffer {
    colour: wgpu::Texture,
    colour_view: wgpu::TextureView,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
}

fn mat_to_gpu(m: Mat4) -> [[f32; 4]; 4] {
    // Mat4 is row-major; WGSL mat4x4 is column-major — transpose before upload.
    m.transpose().m
}

/// wgpu (Vulkan convention) expects NDC depth in [0, 1] with 0 at the near plane.
/// Camera::projection_matrix() uses OpenGL convention (near→−1, far→+1), which causes
/// the near half of the frustum to have z_clip < 0 and be hardware-clipped by wgpu.
/// This matrix maps near→0, far→1 instead.
fn gpu_projection_matrix(camera: &Camera) -> Mat4 {
    let f = 1.0 / (camera.fov * 0.5).tan();
    let nf = 1.0 / (camera.near - camera.far);
    Mat4 {
        m: [
            [f / camera.aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, camera.far * nf, camera.far * camera.near * nf],
            [0.0, 0.0, -1.0, 0.0],
        ],
    }
}

impl GpuRasterRenderer {
    pub fn new() -> Self {
        pollster::block_on(Self::init_async())
    }

    async fn init_async() -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::POLYGON_MODE_LINE,
                ..Default::default()
            })
            .await
            .expect("Failed to get device");

        let pipeline = Self::create_pipeline(&device);
        let wireframe_pipeline = Self::create_wireframe_pipeline(&device);

        Self {
            device,
            queue,
            pipeline,
            wireframe_pipeline,
            colour_texture: RefCell::new(None),
        }
    }

    /// Returns a reference to the offscreen framebuffer, creating or recreating it if the
    /// dimensions have changed.
    fn ensure_framebuffer(&self, w: u32, h: u32) -> std::cell::Ref<GpuFramebuffer> {
        {
            let mut fb = self.colour_texture.borrow_mut();
            let needs_new = fb.as_ref().map_or(true, |f| f.width != w || f.height != h);
            if needs_new {
                *fb = Some(Self::create_gpu_framebuffer(&self.device, w, h));
            }
        }
        std::cell::Ref::map(self.colour_texture.borrow(), |f| f.as_ref().unwrap())
    }

    fn create_gpu_framebuffer(device: &wgpu::Device, w: u32, h: u32) -> GpuFramebuffer {
        let colour = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_colour"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let colour_view = colour.create_view(&Default::default());

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_depth"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&Default::default());

        let bytes_per_row = align_to_256(w * 4);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_buf"),
            size: (bytes_per_row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        GpuFramebuffer {
            colour,
            colour_view,
            depth,
            depth_view,
            readback,
            width: w,
            height: h,
        }
    }

    fn create_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raster.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // LightBlock
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("raster_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,  // position
                        1 => Float32x3,  // normal
                        2 => Float32x2,  // uv
                        3 => Float32x4,  // color
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn create_wireframe_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wireframe_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raster.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Bindings 1-3 are unused by fs_wireframe but must be present to match the layout
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wireframe_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_wireframe"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn upload_object(device: &wgpu::Device, obj: &Object) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let mut verts: Vec<GpuVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for (face_idx, &(i0, i1, i2)) in obj.mesh.faces.iter().enumerate() {
            let (uv_i0, uv_i1, uv_i2) = obj
                .mesh
                .uv_faces
                .get(face_idx)
                .copied()
                .unwrap_or((0, 0, 0));

            let base = verts.len() as u32;
            for (vi, uvi) in [(i0, uv_i0), (i1, uv_i1), (i2, uv_i2)] {
                let pos = obj.mesh.vertices[vi];
                let nor = obj.mesh.normals[vi];
                let uv = obj
                    .mesh
                    .uvs
                    .get(uvi)
                    .copied()
                    .unwrap_or(Vec2::new(0.0, 0.0));
                let color = match &obj.material {
                    Material::Color([r, g, b, a]) => [
                        *r as f32 / 255.0,
                        *g as f32 / 255.0,
                        *b as f32 / 255.0,
                        *a as f32 / 255.0,
                    ],
                    Material::Texture(_) => [1.0, 1.0, 1.0, 1.0],
                };
                verts.push(GpuVertex {
                    position: [pos.x, pos.y, pos.z],
                    normal: [nor.x, nor.y, nor.z],
                    uv: [uv.x, uv.y],
                    color,
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2]);
        }

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buf"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buf"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        (vbuf, ibuf, indices.len() as u32)
    }

    fn build_uniforms(
        device: &wgpu::Device,
        obj: &Object,
        camera: &Camera,
        ambient: f32,
    ) -> wgpu::Buffer {
        let (model, normal_mat) = obj.transform.matrices();
        let data = GpuUniforms {
            model: mat_to_gpu(model),
            view: mat_to_gpu(camera.view_matrix()),
            proj: mat_to_gpu(gpu_projection_matrix(camera)),
            normal_mat: mat_to_gpu(normal_mat),
            cam_pos: [camera.position.x, camera.position.y, camera.position.z, 0.0],
            ambient,
            _pad: [0.0; 3],
        };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&data),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    fn build_light_block(device: &wgpu::Device, lights: &[Arc<dyn Light>]) -> wgpu::Buffer {
        let mut block = GpuLightBlock {
            lights: [GpuLight {
                position:  [0.0; 4],
                color:     [0.0; 4],
                direction: [0.0; 4],
                falloff:   [0.0; 4],
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
                None    => (crate::maths::vec3::Vec3::ZERO, 0.0_f32, 0.0_f32),
            };
            block.lights[i] = GpuLight {
                position:  [p.x, p.y, p.z, intensity],
                color:     [c[0], c[1], c[2], 1.0],
                direction: [dir.x, dir.y, dir.z, cone],
                falloff:   [falloff, 0.0, 0.0, 0.0],
            };
        }
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights"),
            contents: bytemuck::bytes_of(&block),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    fn get_or_create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material: &Material,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let (width, height, rgba): (u32, u32, Vec<u8>) = match material {
            Material::Color([r, g, b, a]) => (1, 1, vec![*r, *g, *b, *a]),
            Material::Texture(tex) => (tex.width, tex.height, {
                let mut data = Vec::with_capacity((tex.width * tex.height * 4) as usize);
                for y in 0..tex.height {
                    for x in 0..tex.width {
                        data.extend_from_slice(
                            &tex.sample(x as f32 / tex.width as f32, y as f32 / tex.height as f32),
                        );
                    }
                }
                data
            }),
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("material_tex"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        (texture, view, sampler)
    }

    fn build_bind_group(
        device: &wgpu::Device,
        pipeline: &wgpu::RenderPipeline,
        uniform_buf: &wgpu::Buffer,
        light_buf: &wgpu::Buffer,
        tex_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("per_object_bg"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

impl super::Renderer for GpuRasterRenderer {
    fn renderer_choice(&self) -> RendererChoice {
        RendererChoice::Gpu
    }

    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> RenderStats {
        let w = framebuffer.width as u32;
        let h = framebuffer.height as u32;

        let gpu_fb = self.ensure_framebuffer(w, h);

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &gpu_fb.colour_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu_fb.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.pipeline);

            for obj in objects {
                let (vbuf, ibuf, index_count) = Self::upload_object(&self.device, obj);
                let uniform_buf = Self::build_uniforms(&self.device, obj, camera, ambient);
                let light_buf = Self::build_light_block(&self.device, lights);
                let (_tex, tex_view, sampler) =
                    Self::get_or_create_texture(&self.device, &self.queue, &obj.material);
                let bind_group = Self::build_bind_group(
                    &self.device,
                    &self.pipeline,
                    &uniform_buf,
                    &light_buf,
                    &tex_view,
                    &sampler,
                );

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        // Copy render texture → readback buffer
        let bytes_per_row = align_to_256(w * 4);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_fb.colour,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &gpu_fb.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let submission_index = self.queue.submit([encoder.finish()]);

        // Map + copy to CPU framebuffer
        let (tx, rx) = std::sync::mpsc::channel();
        gpu_fb
            .readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission_index),
                timeout: None,
            })
            .expect("GPU poll failed");
        rx.recv().unwrap().unwrap();

        {
            let mapped = gpu_fb.readback.slice(..).get_mapped_range();
            for y in 0..h as usize {
                let row_start = y * bytes_per_row as usize;
                let src_row = &mapped[row_start..row_start + w as usize * 4];
                for x in 0..w as usize {
                    let base = x * 4;
                    framebuffer.set_pixel(
                        x,
                        y,
                        [
                            src_row[base],
                            src_row[base + 1],
                            src_row[base + 2],
                            src_row[base + 3],
                        ],
                    );
                }
            }
        }
        gpu_fb.readback.unmap();

        RenderStats {
            triangle_count: objects.iter().map(|o| o.mesh.faces.len()).sum(),
            tile_count: 0,
        }
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> RenderStats {
        let w = framebuffer.width as u32;
        let h = framebuffer.height as u32;

        let gpu_fb = self.ensure_framebuffer(w, h);
        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &gpu_fb.colour_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu_fb.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.wireframe_pipeline);

            for obj in objects {
                let (vbuf, ibuf, index_count) = Self::upload_object(&self.device, obj);
                let uniform_buf = Self::build_uniforms(&self.device, obj, camera, 1.0);
                let light_buf = Self::build_light_block(&self.device, &[]);
                let (_tex, tex_view, sampler) =
                    Self::get_or_create_texture(&self.device, &self.queue, &obj.material);
                let bind_group = Self::build_bind_group(
                    &self.device,
                    &self.wireframe_pipeline,
                    &uniform_buf,
                    &light_buf,
                    &tex_view,
                    &sampler,
                );

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        let bytes_per_row = align_to_256(w * 4);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_fb.colour,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &gpu_fb.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let submission_index = self.queue.submit([encoder.finish()]);

        let (tx, rx) = std::sync::mpsc::channel();
        gpu_fb
            .readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission_index),
                timeout: None,
            })
            .expect("GPU poll failed");
        rx.recv().unwrap().unwrap();

        {
            let mapped = gpu_fb.readback.slice(..).get_mapped_range();
            for y in 0..h as usize {
                let row_start = y * bytes_per_row as usize;
                let src_row = &mapped[row_start..row_start + w as usize * 4];
                for x in 0..w as usize {
                    let base = x * 4;
                    framebuffer.set_pixel(
                        x,
                        y,
                        [
                            src_row[base],
                            src_row[base + 1],
                            src_row[base + 2],
                            src_row[base + 3],
                        ],
                    );
                }
            }
        }
        gpu_fb.readback.unmap();

        RenderStats {
            triangle_count: objects.iter().map(|o| o.mesh.faces.len()).sum(),
            tile_count: 0,
        }
    }
}

fn align_to_256(n: u32) -> u32 {
    (n + 255) & !255
}
