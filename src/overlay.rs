use crate::framebuffer::Framebuffer;
use crate::text::draw_text;
use indexmap::IndexMap;

#[derive(Debug, Default)]
pub struct StatsOverlay {
    stats: IndexMap<String, String>,
}

impl StatsOverlay {
    pub fn add(&mut self, key: &str, value: &str) {
        self.stats.insert(key.to_string(), value.to_string());
    }

    pub fn write_to_framebuffer(&self, framebuffer: &mut Framebuffer) {
        let mut y = 3;
        for (key, val) in &self.stats {
            draw_text(
                framebuffer,
                format!("{}: {}", key, val).as_str(),
                3,
                y,
                [255, 255, 255, 255],
                3,
            );
            y += 20;
        }
    }

    pub fn with_defaults(defaults: Vec<(&str, &str)>) -> Self {
        let mut def = Self::default();
        for (key, val) in defaults {
            def.add(key, val);
        }
        def
    }
}
