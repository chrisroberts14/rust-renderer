use crate::{
    framebuffer::Framebuffer,
    overlay::{fps::FpsCounter, stats_overlay::StatsOverlay},
};

mod fps;
pub mod stats_overlay;
mod text;

/// Manages all the different overlays that can be drawn on top of the rendered image.
pub struct OverlayManager {
    fps_counter: FpsCounter,
    last_frame_time: std::time::Instant,
    pub stats_overlay: StatsOverlay,
}

impl OverlayManager {
    pub fn new(stats_overlay: StatsOverlay) -> Self {
        Self {
            fps_counter: FpsCounter::new(),
            stats_overlay,
            last_frame_time: std::time::Instant::now(),
        }
    }

    pub fn write_to_framebuffer(&mut self, framebuffer: &mut Framebuffer) {
        self.stats_overlay
            .add("fps", &self.fps_counter.fps.to_string());
        self.stats_overlay.write_to_framebuffer(framebuffer);

        let now = std::time::Instant::now();
        self.fps_counter.tick(now - self.last_frame_time);

        self.last_frame_time = now;
    }

    pub fn create_new_stats_overlay(&mut self, defaults: Vec<(&str, &str)>) {
        self.stats_overlay = StatsOverlay::with_defaults(defaults);
    }

    pub fn add_stat(&mut self, key: &str, value: &str) {
        self.stats_overlay.add(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_new_stat() {
        let mut overlay_manager = OverlayManager::new(StatsOverlay::default());
        overlay_manager.add_stat("FPS", "60");
        assert_eq!(
            overlay_manager.stats_overlay.get_stat("FPS"),
            Some(&"60".to_string())
        );
    }

    #[test]
    fn test_create_new_overlay_manager() {
        let stats_manager = StatsOverlay::with_defaults(vec![("test_stat", "10")]);
        let overlay_manager = OverlayManager::new(stats_manager);
        assert_eq!(
            overlay_manager.stats_overlay.get_stat("test_stat"),
            Some(&"10".to_string())
        );
    }

    #[test]
    fn test_create_new_stats_overlay() {
        let mut overlay_manager = OverlayManager::new(StatsOverlay::default());
        overlay_manager.create_new_stats_overlay(vec![("new_stat", "20")]);
        assert_eq!(
            overlay_manager.stats_overlay.get_stat("new_stat"),
            Some(&"20".to_string())
        );
    }
}
