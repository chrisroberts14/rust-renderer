pub mod init_renderer;

use crate::framebuffer::Framebuffer;
use crate::geometry::object::Object;
use crate::scenes::camera::Camera;
use crate::scenes::pointlight::PointLight;

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
        lights: &[PointLight],
        framebuffer: &Framebuffer,
    );

    /// Render all objects as wireframe outlines.
    ///
    /// Called instead of `render_objects` when wireframe mode is active.
    /// The default implementation is a no-op; override to provide wireframe support.
    fn render_wireframe(&self, objects: &[Object], camera: &Camera, framebuffer: &Framebuffer) {
        let _ = (objects, camera, framebuffer);
    }
}
