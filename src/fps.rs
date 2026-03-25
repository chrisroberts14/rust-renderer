use crate::overlay::StatsOverlay;
use std::time::Instant;

pub struct FpsCounter {
    last_time: Instant,
    frame_count: u32,
    fps: u32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }

    pub fn tick(&mut self, overlay: &mut StatsOverlay) {
        self.frame_count += 1;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);

        if elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.last_time = now;
        }
        overlay.add("FPS", &self.fps.to_string());
    }
}
