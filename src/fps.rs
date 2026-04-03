//! A simple FPS counter that tracks the number of frames rendered in the last second.

use std::time::Duration;

pub struct FpsCounter {
    // The total time accumulated since the last FPS update
    accumulated: Duration,
    // The number of frames rendered since the last FPS update
    frame_count: u32,
    // The most recently calculated FPS values
    pub fps: u32,
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

    /// Call this method once per frame, passing in the time elapsed since the last call.
    /// The `fps` field will be updated approximately once per second with the number of frames rendered in that period.
    ///
    /// Passing in the elapsed field rather than calculating it ourselves allows for faking time passing in tests
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
        // 300 frames at 10ms = 3 full seconds; remainder should be zero
        for _ in 0..300 {
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
