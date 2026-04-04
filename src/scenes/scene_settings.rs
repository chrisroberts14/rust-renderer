/// Struct to contain settings about the scene
#[derive(Clone, Default)]
pub struct SceneSettings {
    pub(crate) render_lights: bool,
    pub(crate) wire_frame_mode: bool,
    pub(crate) show_overlay: bool,
}

impl SceneSettings {
    /// Toggle of we show the lights as cubes in the scene useful for debugging
    pub fn toggle_render_lights(&mut self) {
        self.render_lights = !self.render_lights;
    }

    /// Toggle showing wireframe models
    pub fn toggle_wire_frame_mode(&mut self) {
        self.wire_frame_mode = !self.wire_frame_mode;
    }

    /// Toggle showing the debug overlay
    pub(crate) fn toggle_overlay(&mut self) {
        self.show_overlay = !self.show_overlay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_render_lights() {
        let mut settings = SceneSettings::default();
        settings.toggle_render_lights();
        assert!(settings.render_lights);
        settings.toggle_render_lights();
        assert!(!settings.render_lights);
    }

    #[test]
    fn test_toggle_wire_frame_mode() {
        let mut settings = SceneSettings::default();
        settings.toggle_wire_frame_mode();
        assert!(settings.wire_frame_mode);
        settings.toggle_wire_frame_mode();
        assert!(!settings.wire_frame_mode);
    }

    #[test]
    fn test_toggle_overlay() {
        let mut settings = SceneSettings::default();
        settings.toggle_overlay();
        assert!(settings.show_overlay);
        settings.toggle_overlay();
        assert!(!settings.show_overlay);
    }
}
