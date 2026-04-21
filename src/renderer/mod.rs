pub mod cpu;
pub(crate) mod shadow_map;
pub mod vulkan;
pub mod wgsl;

use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::renderer::vulkan::VulkanRenderer;
use crate::renderer::wgsl::WGSLRenderer;
use crate::scenes::camera::Camera;
use crate::scenes::lights::Light;
use crate::scenes::material::Material;
use clap::ValueEnum;
use enum_iter_macro::EnumIter;
use std::fmt;
use std::sync::Arc;
use strum_macros::Display;
use wgpu;

use crate::renderer::cpu::{MultiThreadCPU, SingleThreadCPU};
pub use cpu::shade::shade;

/// CLI argument type for selecting an initial renderer.
/// Once a renderer is implemented it will need to be "registered" here.
#[derive(Clone, ValueEnum, Display, PartialEq, EnumIter)]
pub enum RendererChoice {
    SingleThreadCPU,
    MultiThreadCPU,
    WGSL,
    Vulkan,
}

impl RendererChoice {
    pub fn into_active(self) -> ActiveRenderer {
        match self {
            RendererChoice::SingleThreadCPU => cpu::single_thread_active(),
            RendererChoice::MultiThreadCPU => cpu::multi_thread_active(),
            RendererChoice::WGSL => wgsl::into_active(),
            RendererChoice::Vulkan => vulkan::into_active(),
        }
    }
}

/// The active renderer, wrapping a concrete renderer instance.
///
/// Implements [`Renderer`] by delegating to the inner type, and exposes variant-specific
/// operations (tile count, GPU view) directly — so callers never need to runtime-check the
/// variant just to call a method.
pub enum ActiveRenderer {
    SingleThreadCPU(Box<SingleThreadCPU>),
    MultiThreadCPU(Box<MultiThreadCPU>),
    WGSL(Box<WGSLRenderer>),
    Vulkan(Box<VulkanRenderer>),
}

impl ActiveRenderer {
    fn as_choice(&self) -> RendererChoice {
        match self {
            Self::SingleThreadCPU(_) => RendererChoice::SingleThreadCPU,
            Self::MultiThreadCPU(_) => RendererChoice::MultiThreadCPU,
            Self::WGSL(_) => RendererChoice::WGSL,
            Self::Vulkan(_) => RendererChoice::Vulkan,
        }
    }

    /// Cycles to the next renderer in the sequence, replacing the current one in place.
    /// Order follows [`RendererChoice::iter`], so new variants slot in automatically.
    pub fn next(&mut self) {
        let choices: Vec<_> = RendererChoice::iter().collect();
        let current = self.as_choice();
        let idx = choices.iter().position(|c| c == &current).unwrap_or(0);
        *self = choices[(idx + 1) % choices.len()].clone().into_active();
    }

    /// Returns the GPU colour texture view from the most recent render, or `None` for CPU renderers.
    pub fn take_gpu_view(&self) -> Option<wgpu::TextureView> {
        match self {
            Self::WGSL(r) => r.take_gpu_view(),
            _ => None,
        }
    }

    pub fn take_vk_image(&self) -> Option<Arc<vulkano::image::Image>> {
        match self {
            Self::Vulkan(r) => r.take_vk_image(),
            _ => None,
        }
    }

    /// Increases the tile count. No-op on the GPU renderer.
    pub fn increase_tile_count(&mut self, delta: usize) {
        match self {
            Self::SingleThreadCPU(r) => r.increase_tile_count(delta),
            Self::MultiThreadCPU(r) => r.increase_tile_count(delta),
            Self::WGSL(_) | Self::Vulkan(_) => {}
        }
    }

    /// Decreases the tile count. No-op on the GPU renderer.
    pub fn decrease_tile_count(&mut self, delta: usize) {
        match self {
            Self::SingleThreadCPU(r) => r.decrease_tile_count(delta),
            Self::MultiThreadCPU(r) => r.decrease_tile_count(delta),
            Self::WGSL(_) | Self::Vulkan(_) => {}
        }
    }
}

impl Renderer for ActiveRenderer {
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)> {
        match self {
            Self::SingleThreadCPU(r) => {
                r.render_objects(objects, camera, lights, framebuffer, ambient)
            }
            Self::MultiThreadCPU(r) => {
                r.render_objects(objects, camera, lights, framebuffer, ambient)
            }
            Self::WGSL(r) => r.render_objects(objects, camera, lights, framebuffer, ambient),
            Self::Vulkan(r) => r.render_objects(objects, camera, lights, framebuffer, ambient),
        }
    }

    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)> {
        match self {
            Self::SingleThreadCPU(r) => r.render_wireframe(objects, camera, framebuffer),
            Self::MultiThreadCPU(r) => r.render_wireframe(objects, camera, framebuffer),
            Self::WGSL(r) => r.render_wireframe(objects, camera, framebuffer),
            Self::Vulkan(r) => r.render_wireframe(objects, camera, framebuffer),
        }
    }
}

impl fmt::Display for ActiveRenderer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleThreadCPU(_) => write!(f, "SingleThreadCPU"),
            Self::MultiThreadCPU(_) => write!(f, "MultiThreadCPU"),
            Self::WGSL(_) => write!(f, "WGSL"),
            Self::Vulkan(_) => write!(f, "Vulkan"),
        }
    }
}

/// The interface that all renderers must implement.
///
/// A renderer is responsible for turning a set of scene objects into pixels in a framebuffer.
/// The framebuffer is not cleared by any of these methods — the caller is responsible for
/// pre-filling it (e.g. with a skybox) before invoking the renderer.
pub trait Renderer {
    /// Render all objects into the framebuffer using the given camera and lights.
    fn render_objects(
        &self,
        objects: &[Object],
        camera: &Camera,
        lights: &[Arc<dyn Light>],
        framebuffer: &Framebuffer,
        ambient: f32,
    ) -> Vec<(&'static str, String)>;

    /// Render all objects as wireframe outlines.
    ///
    /// Called instead of `render_objects` when wireframe mode is active.
    fn render_wireframe(
        &self,
        objects: &[Object],
        camera: &Camera,
        framebuffer: &Framebuffer,
    ) -> Vec<(&'static str, String)>;
}

/// A vertex bundle: (camera-space position, world-space position, world-space normal, texture UV)
#[derive(Clone, Copy)]
struct Vert {
    cam: Vec3,
    world: Vec3,
    normal: Vec3,
    uv: Vec2,
}

/// A triangle with everything needed to rasterize
pub struct PreparedTriangle {
    verts: [Vert; 3],
    screen: [Vec2; 3],
    depths: [f32; 3],
    material: Material,
    is_light: bool,
}
