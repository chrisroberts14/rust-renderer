/// Struct to contain settings about the scene
#[derive(Clone, Default, Debug)]
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

    /// Return scene settings as string pairs
    pub(crate) fn as_pairs(&self) -> Vec<(String, String)> {
        vec![
            ("render_lights".to_string(), self.render_lights.to_string()),
            (
                "wire_frame_mode".to_string(),
                self.wire_frame_mode.to_string(),
            ),
        ]
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

    #[test]
    fn test_as_pairs() {
        let mut settings = SceneSettings::default();
        settings.render_lights = true;
        settings.wire_frame_mode = true;
        let pairs = settings.as_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs.get(0),
            Some(&("render_lights".to_string(), "true".to_string()))
        );
        assert_eq!(
            pairs.get(1),
            Some(&("wire_frame_mode".to_string(), "true".to_string()))
        );
    }
}
