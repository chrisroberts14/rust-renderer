/// Struct to contain settings about the scene
#[derive(Clone)]
pub(crate) struct SceneSettings {
    pub(crate) render_lights: bool,
    pub(crate) wire_frame_mode: bool,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneSettings {
    pub(crate) fn new() -> Self {
        Self {
            render_lights: false,
            wire_frame_mode: false,
        }
    }

    /// Toggle of we show the lights as cubes in the scene useful for debugging
    pub(crate) fn toggle_render_lights(&mut self) {
        self.render_lights = !self.render_lights;
    }

    /// Toggle showing wireframe models
    pub(crate) fn toggle_wire_frame_mode(&mut self) {
        self.wire_frame_mode = !self.wire_frame_mode;
    }
}
