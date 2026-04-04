//! Owns the wgpu surface and all presentation machinery. This was originally a replacement for
//! the pixels crate so we can bypass the CPU for GPU-rendered frames
//!
//! [`DisplaySurface`] is the single point of contact between the renderers and the screen. It
//! holds the wgpu instance, surface, device, and queue, and exposes two presentation methods:
//!
//! - [`present_cpu_frame`](DisplaySurface::present_cpu_frame) — used by the CPU raster renderers.
//!   Uploads a raw RGBA byte slice to a staging texture then blits it to the swap chain.
//! - [`present_gpu_frame`](DisplaySurface::present_gpu_frame) — used by the GPU renderer. Blits
//!   an already-rendered offscreen texture directly to the swap chain with no CPU roundtrip.
//!   An optional `overlay_bytes` slice (RGBA8, transparent background) can be provided; when
//!   present it is uploaded and composited on top in the same single render pass via the shader.
//!
//! The device and queue are wrapped in [`Arc`] so the GPU renderer can share them via
//! [`shared_device`](DisplaySurface::shared_device) / [`shared_queue`](DisplaySurface::shared_queue),
//! which is required for GPU-rendered textures to be usable on the same device as the surface.
//!
//! The surface format is chosen to be non-sRGB where possible so that pixel values are written
//! directly to the screen without any implicit gamma encoding.

use std::sync::Arc;
use winit::window::{CursorGrabMode, Window};

pub struct DisplaySurface<'window> {
    /// The window is managed here which ensures it lives at least as long as the surface
    window: Option<Arc<dyn Window>>,
    /// The wgpu instance used to create the surface and adapter.
    pub instance: wgpu::Instance,
    /// The wgpu surface backed by the winit window.
    pub surface: wgpu::Surface<'window>,
    /// Shared with [`GpuRasterRenderer`](crate::renderer::gpu_raster_renderer::GpuRasterRenderer)
    /// so that GPU-rendered textures can be presented without crossing device boundaries.
    pub device: Arc<wgpu::Device>,
    /// Shared queue — see `device` above.
    pub queue: Arc<wgpu::Queue>,
    /// Swap-chain format selected at init time (non-sRGB preferred).
    pub surface_format: wgpu::TextureFormat,
    /// Swap-chain configuration; mutated by [`resize`](DisplaySurface::resize).
    config: wgpu::SurfaceConfiguration,
    /// Single fullscreen-triangle blit pipeline — compositing is handled in the shader.
    blit_pipeline: wgpu::RenderPipeline,
    /// Bind group layout: binding 0 = frame texture, binding 1 = overlay texture, binding 2 = sampler.
    bind_group_layout: wgpu::BindGroupLayout,
    /// Staging texture for CPU-side pixel data (CPU renderer output or overlay bytes).
    cpu_texture: wgpu::Texture,
    /// Permanent 1×1 fully-transparent texture used as the overlay when none is provided.
    null_overlay: wgpu::Texture,
    /// Nearest-neighbour sampler used by the blit pass.
    sampler: wgpu::Sampler,
}

impl<'window> DisplaySurface<'window> {
    /// Creates a `DisplaySurface` for the given window, blocking until the wgpu adapter and device
    /// are ready. `width` and `height` are the initial surface dimensions in physical pixels.
    pub fn new(window: Arc<dyn Window>, width: usize, height: usize) -> Self {
        pollster::block_on(Self::init_async(window, width, height))
    }

    async fn init_async(window: Arc<dyn Window>, width: usize, height: usize) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
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

        let surface_caps = surface.get_capabilities(&adapter);
        // Prefer non-sRGB so pixel values are written directly without implicit gamma encoding.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: width as u32,
            height: height as u32,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Closure to reduce repetition for the two identical texture binding entries.
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit_bind_group_layout"),
            entries: &[
                tex_entry(0),
                tex_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // 1×1 transparent texture bound as the overlay when the caller passes None.
        let null_overlay = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("null_overlay"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &null_overlay,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        Self {
            window: Some(window),
            cpu_texture: Self::create_cpu_texture(&device, width as u32, height as u32),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
            blit_pipeline,
            null_overlay,
            instance,
            surface,
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface_format,
            config,
            bind_group_layout,
        }
    }

    /// Reconfigures the surface and recreates the CPU upload texture for the new dimensions.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.cpu_texture = Self::create_cpu_texture(&self.device, width, height);
    }

    /// Uploads raw RGBA bytes from the CPU framebuffer and blits them to the surface.
    pub fn present_cpu_frame(&self, pixels: &[u8]) {
        self.upload_to_cpu_texture(pixels);
        let view = self.cpu_texture.create_view(&Default::default());
        self.present_gpu_frame(&view, None);
    }

    /// Blits a GPU texture view to the surface.
    ///
    /// If `overlay_bytes` is `Some`, it must be RGBA8 pixels where background pixels are
    /// `[0,0,0,0]` (transparent). They are uploaded and alpha-composited on top in the same
    /// render pass via `mix(frame, overlay, overlay.a)` in the shader.
    pub fn present_gpu_frame(&self, gpu_view: &wgpu::TextureView, overlay_bytes: Option<&[u8]>) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let surface_view = surface_texture.texture.create_view(&Default::default());

        let overlay_view = if let Some(overlay) = overlay_bytes {
            self.upload_to_cpu_texture(overlay);
            self.cpu_texture.create_view(&Default::default())
        } else {
            self.null_overlay.create_view(&Default::default())
        };

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(gpu_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&overlay_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    pub fn shared_device(&self) -> Arc<wgpu::Device> {
        Arc::clone(&self.device)
    }
    pub fn shared_queue(&self) -> Arc<wgpu::Queue> {
        Arc::clone(&self.queue)
    }

    fn create_cpu_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cpu_framebuffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn upload_to_cpu_texture(&self, bytes: &[u8]) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.cpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.config.width * 4),
                rows_per_image: Some(self.config.height),
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn release_mouse(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
            window.set_cursor_grab(CursorGrabMode::None)?;
            Ok(())
        } else {
            Err("Window not initialized".into())
        }
    }

    pub fn capture_mouse(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(window) = &self.window {
            window.set_cursor_visible(false);
            window.set_cursor_grab(CursorGrabMode::Confined)?;
            Ok(())
        } else {
            Err("Window not initialized".into())
        }
    }

    pub fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
