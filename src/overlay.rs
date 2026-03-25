use crate::framebuffer::Framebuffer;
use crate::text::draw_text;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct StatsOverlay {
    stats: HashMap<String, String>,
}

impl StatsOverlay {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: &str, value: &str) {
        self.stats.insert(key.to_string(), value.to_string());
    }

    pub fn stat(&self, key: &str) -> Option<&String> {
        self.stats.get(key)
    }

    pub fn write_to_framebuffer(&self, framebuffer: &mut Framebuffer) {
        let mut y = 3;
        self.stats.iter().for_each(|(key, value)| {
            draw_text(
                framebuffer,
                format!("{}: {}", key, value).as_str(),
                3,
                y,
                [255, 255, 255, 255],
                3,
            );
            y += 1;
        });
    }
}
