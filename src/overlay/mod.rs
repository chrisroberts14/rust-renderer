use crate::{framebuffer::Framebuffer, overlay::stats_overlay::StatsOverlay};

pub mod stats_overlay;
mod text;

/// Manages all the different overlays that can be drawn on top of the rendered image.
pub struct OverlayManager {
    pub stats_overlay: StatsOverlay,
}

impl OverlayManager {
    pub fn new(stats_overlay: StatsOverlay) -> Self {
        Self { stats_overlay }
    }

    pub fn write_to_framebuffer(&self, framebuffer: &mut Framebuffer) {
        self.stats_overlay.write_to_framebuffer(framebuffer);
    }

    pub fn create_new_stats_overlay(&mut self, defaults: Vec<(&str, &str)>) {
        self.stats_overlay = StatsOverlay::with_defaults(defaults);
    }

    pub fn add_stat(&mut self, key: &str, value: &str) {
        self.stats_overlay.add(key, value);
    }
}
