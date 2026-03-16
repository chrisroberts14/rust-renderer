use std::time::{Duration, Instant};

pub struct FpsCounter {
    last_time: Instant,
    frame_count: u32,
    pub fps: u32
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);
        println!("FPS: {}", self.fps);

        if elapsed.as_secs_f32() >= 1.0 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.last_time = now;
        }
    }
}