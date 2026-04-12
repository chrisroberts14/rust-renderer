/// A renderer that rasterizes geometry on the GPU via wgpu, then reads the pixels back to a CPU
/// [`Framebuffer`] so it is compatible with the rest of the rendering pipeline.
///
use crate::display::DisplaySurface;
use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use std::cell::RefCell;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// There needs to be a maximum number of lights as we need fixed size arrays
const MAX_LIGHTS: usize = 8;
const SHADOW_MAP_GPU_SIZE: u32 = 512;

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
    position: [f32; 4],  // xyz = world pos, w = intensity
    color: [f32; 4],     // xyz = rgb, w = unused
    direction: [f32; 4], // xyz = spot direction, w = cone_angle (0 = point light)
    falloff: [f32; 4],   // x = falloff_angle, yzw = padding
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLightBlock {
    lights: [GpuLight; MAX_LIGHTS],
    light_count: u32,
    _pad: [u32; 3],
}

/// Per-draw-call uniforms for the depth-only shadow pass.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShadowPassUniforms {
    light_vp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

/// One view-projection matrix per light, sent to the main shader for shadow lookups.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShadowBlock {
    light_vp: [[[f32; 4]; 4]; MAX_LIGHTS],
}

struct ShadowBindings<'a> {
    view: &'a wgpu::TextureView,
    sampler: &'a wgpu::Sampler,
    block_buf: &'a wgpu::Buffer,
}

pub struct GpuRasterRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    // RefCell needed because render_objects takes &self but we lazily initialise/resize this
    colour_texture: RefCell<Option<GpuFramebuffer>>,
    last_colour_view: RefCell<Option<wgpu::TextureView>>,
}

struct GpuFramebuffer {
    colour: wgpu::Texture,
    colour_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
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

/// Builds a light view-projection matrix using the GPU depth convention (near→0, far→1).
/// Mirrors the view-matrix layout of `Camera::view_matrix` for consistency.
fn light_gpu_view_proj(light: &dyn Light, near: f32, far: f32) -> Mat4 {
    let pos = light.position();

    let forward = if let Some(dir) = light.spot_direction() {
        dir.normalise()
    } else {
        let to_origin = Vec3::ZERO - pos;
        if to_origin.length() < 0.001 {
            Vec3::new(0.0, -1.0, 0.0)
        } else {
            to_origin.normalise()
        }
    };

    let up_hint = if forward.dot(Vec3::new(0.0, 1.0, 0.0)).abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    let r = up_hint.cross(forward).normalise();
    let u = forward.cross(r).normalise();
    let view = Mat4 {
        m: [
            [r.x, r.y, r.z, -r.dot(pos)],
            [u.x, u.y, u.z, -u.dot(pos)],
            [-forward.x, -forward.y, -forward.z, forward.dot(pos)],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    let fov = if light.spot_direction().is_some() {
        ((light.cone_angle() + light.falloff_angle()) * 2.0).max(0.1)
    } else {
        std::f32::consts::FRAC_PI_2
    };

    // GPU-compatible perspective: maps near → 0, far → 1.
    let f = 1.0 / (fov * 0.5).tan();
    let nf = 1.0 / (near - far);
    let proj = Mat4 {
        m: [
            [f, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far * nf, far * near * nf],
            [0.0, 0.0, -1.0, 0.0],
        ],
    };

    proj * view
}

impl Default for GpuRasterRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuRasterRenderer {
    /// Creates the renderer, blocking the calling thread until the wgpu device is ready.
    pub fn new() -> Self {
        pollster::block_on(Self::init_async())
    }

    /// Requests a high-performance adapter and device, then compiles all render pipelines.
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

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let bind_group_layout = Self::create_bind_group_layout(&device);
        let shadow_bind_group_layout = Self::create_shadow_bind_group_layout(&device);
        let pipeline = Self::create_pipeline(&device, &bind_group_layout, false);
        let wireframe_pipeline = Self::create_pipeline(&device, &bind_group_layout, true);
        let shadow_pipeline = Self::create_shadow_pipeline(&device, &shadow_bind_group_layout);

        Self {
            device,
            queue,
            pipeline,
            wireframe_pipeline,
            shadow_pipeline,
            bind_group_layout,
            shadow_bind_group_layout,
            colour_texture: RefCell::new(None),
            last_colour_view: RefCell::new(None),
        }
    }

    /// Creates a renderer that shares the wgpu device and queue with the given display surface.
    /// This allows the GPU renderer to blit its output directly to the surface without a CPU readback.
    pub fn from_display(display: &DisplaySurface<'_>) -> Self {
        let device = display.shared_device();
        let queue = display.shared_queue();

        let bind_group_layout = Self::create_bind_group_layout(&device);
        let shadow_bind_group_layout = Self::create_shadow_bind_group_layout(&device);
        let pipeline = Self::create_pipeline(&device, &bind_group_layout, false);
        let wireframe_pipeline = Self::create_pipeline(&device, &bind_group_layout, true);
        let shadow_pipeline = Self::create_shadow_pipeline(&device, &shadow_bind_group_layout);

        Self {
            device,
            queue,
            pipeline,
            wireframe_pipeline,
            shadow_pipeline,
            bind_group_layout,
            shadow_bind_group_layout,
            colour_texture: RefCell::new(None),
            last_colour_view: RefCell::new(None),
        }
    }

    /// Returns a reference to the offscreen framebuffer, creating or recreating it if the
    /// dimensions have changed.
    fn ensure_framebuffer(&self, w: u32, h: u32) -> std::cell::Ref<'_, GpuFramebuffer> {
        {
            let mut fb = self.colour_texture.borrow_mut();
            let needs_new = fb.as_ref().is_none_or(|f| f.width != w || f.height != h);
            if needs_new {
                *fb = Some(Self::create_gpu_framebuffer(&self.device, w, h));
            }
        }
        std::cell::Ref::map(self.colour_texture.borrow(), |f| f.as_ref().unwrap())
    }

    /// Allocates the offscreen colour texture and depth texture for a framebuffer of the given dimensions.
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
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

        GpuFramebuffer {
            colour,
            colour_view,
            depth_view,
            width: w,
            height: h,
        }
    }

    /// Creates the shared bind group layout used by both main render pipelines.
    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // @binding(0) — per-object transform + camera uniforms
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
                // @binding(1) — scene lights
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
                // @binding(2) — material texture
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
                // @binding(3) — material sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // @binding(4) — shadow map depth array (one layer per light)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // @binding(5) — comparison sampler for PCF shadow lookups
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // @binding(6) — light view-projection matrices for shadow sampling
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    /// Bind group layout for the depth-only shadow pass: a single uniform with light VP + model.
    fn create_shadow_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    /// Builds a render pipeline. When `wireframe` is true, uses `fs_wireframe` and
    /// `PolygonMode::Line`; otherwise uses `fs_main` with back-face culling.
    fn create_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        wireframe: bool,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raster.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(if wireframe {
                "wireframe_pipeline"
            } else {
                "raster_pipeline"
            }),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, // position
                        1 => Float32x3, // normal
                        2 => Float32x2, // uv
                        3 => Float32x4, // color
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if wireframe { "fs_wireframe" } else { "fs_main" }),
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
                polygon_mode: if wireframe {
                    wgpu::PolygonMode::Line
                } else {
                    wgpu::PolygonMode::Fill
                },
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

    /// Builds the depth-only pipeline used for shadow map rendering.
    fn create_shadow_pipeline(
        device: &wgpu::Device,
        shadow_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[Some(shadow_bind_group_layout)],
            immediate_size: 0,
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                // Reuse GpuVertex buffers; only position (@location 0) is read.
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: None, // depth-only — no colour output
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
                // Slope-scale bias prevents self-shadowing (shadow acne) without PCF.
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    /// Converts an [`Object`]'s mesh into interleaved [`GpuVertex`] data and uploads it to a
    /// vertex buffer and index buffer. Returns both buffers and the index count.
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

    /// Packs per-object transform matrices, camera data, and ambient intensity into a uniform
    /// buffer. Matrices are transposed from row-major (Rust) to column-major (WGSL).
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

    /// Packs all scene lights into a `GpuLightBlock` uniform buffer (up to `MAX_LIGHTS`).
    /// Point lights have `direction.w == 0`; spot lights carry their cone and falloff angles.
    fn build_light_block(device: &wgpu::Device, lights: &[Arc<dyn Light>]) -> wgpu::Buffer {
        let mut block = GpuLightBlock {
            lights: [GpuLight {
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
            block.lights[i] = GpuLight {
                position: [p.x, p.y, p.z, intensity],
                color: [c[0], c[1], c[2], 1.0],
                direction: [dir.x, dir.y, dir.z, cone],
                falloff: [falloff, 0.0, 0.0, 0.0],
            };
        }
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights"),
            contents: bytemuck::bytes_of(&block),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    /// Uploads the object's material as a GPU texture. For `Material::Color` a 1×1 texture is
    /// created; for `Material::Texture` the full image is rasterised row-by-row and uploaded.
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

    /// Creates the bind group for a single draw call, wiring all seven bindings to their shader slots.
    fn build_bind_group(
        &self,
        uniform_buf: &wgpu::Buffer,
        light_buf: &wgpu::Buffer,
        tex_view: &wgpu::TextureView,
        tex_sampler: &wgpu::Sampler,
        shadow: &ShadowBindings<'_>,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("per_object_bg"),
            layout: &self.bind_group_layout,
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
                    resource: wgpu::BindingResource::Sampler(tex_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(shadow.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(shadow.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: shadow.block_buf.as_entire_binding(),
                },
            ],
        })
    }

    /// Creates a `Depth32Float` 2D-array texture with `MAX_LIGHTS` layers used for shadow maps.
    /// Pass `size = 1` to produce a cheap dummy texture for wireframe / no-light rendering.
    fn create_shadow_texture(&self, size: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_map_array"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: MAX_LIGHTS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// Encodes one depth-only render pass per light into `encoder`, writing results into the
    /// corresponding layer of a `MAX_LIGHTS`-layer shadow texture array.
    /// Returns the shadow texture and the `GpuShadowBlock` of light VP matrices for the shader.
    fn build_shadow_maps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        lights: &[Arc<dyn Light>],
        objects: &[Object],
        camera: &Camera,
    ) -> (wgpu::Texture, GpuShadowBlock) {
        let shadow_texture = self.create_shadow_texture(SHADOW_MAP_GPU_SIZE);
        let mut shadow_block = GpuShadowBlock {
            light_vp: [[[0.0f32; 4]; 4]; MAX_LIGHTS],
        };

        for (light_idx, light) in lights.iter().take(MAX_LIGHTS).enumerate() {
            let lv_proj = light_gpu_view_proj(light.as_ref(), camera.near, camera.far);
            shadow_block.light_vp[light_idx] = mat_to_gpu(lv_proj);

            // View into this light's layer of the shadow texture array.
            let layer_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("shadow_layer_view"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: light_idx as u32,
                array_layer_count: Some(1),
                ..Default::default()
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &layer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.shadow_pipeline);

            for obj in objects {
                if obj.is_light {
                    continue;
                }
                let (vbuf, ibuf, index_count) = Self::upload_object(&self.device, obj);
                let (model, _) = obj.transform.matrices();
                let shadow_uniforms = GpuShadowPassUniforms {
                    light_vp: mat_to_gpu(lv_proj),
                    model: mat_to_gpu(model),
                };
                let uniform_buf =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("shadow_pass_uniforms"),
                            contents: bytemuck::bytes_of(&shadow_uniforms),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shadow_pass_bg"),
                    layout: &self.shadow_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buf.as_entire_binding(),
                    }],
                });
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        (shadow_texture, shadow_block)
    }

    /// Internal render method shared by `render_objects` and `render_wireframe`.
    /// When `wireframe` is true the wireframe pipeline is used and lights/shadows are ignored.
    fn render_scene(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
        wireframe: bool,
    ) -> Vec<(&'static str, String)> {
        let (w, h) = (framebuffer.width as u32, framebuffer.height as u32);
        let gpu_fb = self.ensure_framebuffer(w, h);
        let pipeline = if wireframe {
            &self.wireframe_pipeline
        } else {
            &self.pipeline
        };

        // Seed the GPU colour texture from the CPU framebuffer so that the skybox
        // (drawn directly to the CPU framebuffer) is preserved in the GPU render pass.
        // write_texture flushes before the next submit, so the render pass LoadOp::Load sees it.
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu_fb.colour,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            framebuffer.as_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Shadow pass: skip during wireframe, or when there are no lights.
        let (shadow_texture, shadow_block) = if !wireframe && !lights.is_empty() {
            self.build_shadow_maps(&mut encoder, lights, objects, camera)
        } else {
            // Dummy 1×1 texture — never sampled (no lights or wireframe mode).
            (
                self.create_shadow_texture(1),
                GpuShadowBlock {
                    light_vp: [[[0.0f32; 4]; 4]; MAX_LIGHTS],
                },
            )
        };

        let shadow_array_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let shadow_comparison_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_comparison_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_block_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shadow_block"),
                contents: bytemuck::bytes_of(&shadow_block),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &gpu_fb.colour_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

            pass.set_pipeline(pipeline);
            let light_buf = Self::build_light_block(&self.device, lights);
            let empty_light_buf = Self::build_light_block(&self.device, &[]);
            for obj in objects {
                let (vbuf, ibuf, index_count) = Self::upload_object(&self.device, obj);
                let uniform_buf = Self::build_uniforms(&self.device, obj, camera, ambient);
                let (_tex, tex_view, tex_sampler) =
                    Self::get_or_create_texture(&self.device, &self.queue, &obj.material);
                let active_light_buf = if obj.is_light {
                    &empty_light_buf
                } else {
                    &light_buf
                };
                let bind_group = self.build_bind_group(
                    &uniform_buf,
                    active_light_buf,
                    &tex_view,
                    &tex_sampler,
                    &ShadowBindings {
                        view: &shadow_array_view,
                        sampler: &shadow_comparison_sampler,
                        block_buf: &shadow_block_buf,
                    },
                );

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(0, vbuf.slice(..));
                pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..index_count, 0, 0..1);
            }
        }

        // Create the view before dropping the Ref borrow on gpu_fb
        let colour_view = gpu_fb.colour.create_view(&Default::default());
        drop(gpu_fb);

        self.queue.submit([encoder.finish()]);
        *self.last_colour_view.borrow_mut() = Some(colour_view);

        vec![(
            "Triangle Count",
            objects
                .iter()
                .map(|o| o.mesh.faces.len())
                .sum::<usize>()
                .to_string(),
        )]
    }

    pub fn take_gpu_view(&self) -> Option<wgpu::TextureView> {
        self.last_colour_view.borrow_mut().take()
    }
}

impl super::Renderer for GpuRasterRenderer {
    /// Renders all objects with Phong shading and shadow maps, copies the result from the GPU
    /// to the CPU framebuffer, and returns triangle statistics.
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        self.render_scene(objects, camera, lights, framebuffer, ambient, false)
    }

    /// Renders all objects as flat-white wireframes using `PolygonMode::Line`, then copies the
    /// result to the CPU framebuffer.
    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        self.render_scene(objects, camera, &[], framebuffer, 1.0, true)
    }
}
