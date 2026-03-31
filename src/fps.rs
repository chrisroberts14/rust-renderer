use crate::overlay::StatsOverlay;
use std::time::Duration;

pub struct FpsCounter {
    accumulated: Duration,
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
            accumulated: Duration::ZERO,
            frame_count: 0,
            fps: 0,
        }
    }

    pub fn tick(&mut self, elapsed: Duration, overlay: &mut StatsOverlay) {
        self.frame_count += 1;
        self.accumulated += elapsed;

        if self.accumulated >= Duration::from_secs(1) {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.accumulated -= Duration::from_secs(1);
        }
        overlay.add("FPS", &self.fps.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_fps(overlay: &StatsOverlay) -> u32 {
        overlay
            .entries()
            .find(|(k, _)| *k == "FPS")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap()
    }

    #[test]
    fn starts_at_zero() {
        let mut counter = FpsCounter::new();
        let mut overlay = StatsOverlay::default();
        counter.tick(Duration::from_millis(16), &mut overlay);
        assert_eq!(get_fps(&overlay), 0);
    }

    #[test]
    fn reports_count_after_one_second() {
        let mut counter = FpsCounter::new();
        let mut overlay = StatsOverlay::default();
        // 100 frames at 10ms each = exactly 1 second
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10), &mut overlay);
        }
        assert_eq!(get_fps(&overlay), 100);
    }

    #[test]
    fn accumulator_does_not_drift_over_multiple_seconds() {
        let mut counter = FpsCounter::new();
        let mut overlay = StatsOverlay::default();
        // 300 frames at 10ms = 3 full seconds; remainder should be zero
        for _ in 0..300 {
            counter.tick(Duration::from_millis(10), &mut overlay);
        }
        assert_eq!(counter.accumulated, Duration::ZERO);
        assert_eq!(get_fps(&overlay), 100);
    }

    #[test]
    fn window_resets_after_boundary() {
        let mut counter = FpsCounter::new();
        let mut overlay = StatsOverlay::default();
        // First second: exactly 100 frames at 10ms each
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10), &mut overlay);
        }
        assert_eq!(get_fps(&overlay), 100);
        // Second second: exactly 50 frames at 20ms each
        for _ in 0..50 {
            counter.tick(Duration::from_millis(20), &mut overlay);
        }
        assert_eq!(get_fps(&overlay), 50);
    }
}
