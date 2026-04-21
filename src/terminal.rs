//! Display on the terminal mostly used for debug output

use std::time::{Duration, Instant};
use terminal_display::{
    Block, BoxedWidget, Constraint, Terminal, TerminalHandle, Text, VStack, WidgetExt,
};

struct FpsCounter {
    accumulated: Duration,
    frame_count: u32,
    fps: u32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            accumulated: Duration::ZERO,
            frame_count: 0,
            fps: 0,
        }
    }

    fn tick(&mut self, elapsed: Duration) {
        self.frame_count += 1;
        self.accumulated += elapsed;
        while self.accumulated >= Duration::from_secs(1) {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.accumulated -= Duration::from_secs(1);
        }
    }
}

/// Terminal display for stats
pub struct StatsDisplay {
    terminal_handle: TerminalHandle,
    fps_counter: FpsCounter,
    last_frame_time: Instant,
}

impl StatsDisplay {
    pub fn new() -> StatsDisplay {
        let terminal = Terminal::new().expect("Failed to initialize terminal");
        let terminal_handle = terminal.run();
        render_lines(&terminal_handle, vec!["fps: 0".to_string()]);
        Self {
            terminal_handle,
            fps_counter: FpsCounter::new(),
            last_frame_time: Instant::now(),
        }
    }

    /// Call once per frame with the current render stats and scene settings as
    /// pre-formatted "key: value" strings. Prepends the current fps automatically.
    pub fn update(&mut self, lines: Vec<String>) {
        let now = Instant::now();
        self.fps_counter.tick(now - self.last_frame_time);
        self.last_frame_time = now;

        let mut all_lines = vec![format!("fps: {}", self.fps_counter.fps)];
        all_lines.extend(lines);

        render_lines(&self.terminal_handle, all_lines);
    }
}

fn render_lines(handle: &TerminalHandle, lines: Vec<String>) {
    handle.render(move |frame| {
        let area = frame.area();
        let outer = Block::new().title("Stats");
        let inner = outer.inner(area);
        frame.render(outer, area);

        let rows: Vec<(Constraint, BoxedWidget)> = lines
            .into_iter()
            .map(|line| Text::raw(line).fixed(1))
            .collect();

        frame.render(VStack::new(rows), inner);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_counter_reports_count_after_one_second() {
        let mut counter = FpsCounter::new();
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.fps, 100);
    }

    #[test]
    fn fps_counter_resets_after_boundary() {
        let mut counter = FpsCounter::new();
        for _ in 0..100 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.fps, 100);
        for _ in 0..50 {
            counter.tick(Duration::from_millis(20));
        }
        assert_eq!(counter.fps, 50);
    }

    #[test]
    fn fps_counter_accumulator_does_not_drift() {
        let mut counter = FpsCounter::new();
        for _ in 0..3000 {
            counter.tick(Duration::from_millis(10));
        }
        assert_eq!(counter.accumulated, Duration::ZERO);
        assert_eq!(counter.fps, 100);
    }
}
