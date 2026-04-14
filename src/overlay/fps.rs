use std::time::Duration;

/// Tracks frames-per-second by accumulating frame durations.
/// Call [`tick`](FpsCounter::tick) once per frame; read [`fps`](FpsCounter::fps) for the last completed second's count.
#[derive(Default)]
pub struct FpsCounter {
    accumulated: Duration,
    frame_count: u32,
    pub fps: u32,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pass the elapsed time since the last frame. Accepts an explicit duration so tests can fake time.
    pub fn tick(&mut self, elapsed: Duration) {
        self.frame_count += 1;
        self.accumulated += elapsed;

        if self.accumulated >= Duration::from_secs(1) {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.accumulated -= Duration::from_secs(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_count_after_one_second() {
        let mut counter = FpsCounter::new();
        // 100 frames at 10ms each = exactly 1 second
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.fps, 100);
    }

    #[test]
    fn accumulator_does_not_drift_over_multiple_seconds() {
        let mut counter = FpsCounter::new();
        // 3000 frames at 10ms = 30 full seconds; remainder should be zero
        for _ in 0..3000 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.accumulated, Duration::ZERO);
        assert_eq!(counter.fps, 100);
    }

    #[test]
    fn window_resets_after_boundary() {
        let mut counter = FpsCounter::new();
        // First second: exactly 100 frames at 10ms each
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.fps, 100);
        // Second second: exactly 50 frames at 20ms each
        for _ in 0..50 {
            counter.tick(Duration::from_millis(20));
        }
        assert_eq!(counter.fps, 50);
    }
}
