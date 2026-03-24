/// Struct to contain settings about the scene
#[derive(Clone)]
pub(crate) struct SceneSettings {
    pub render_lights: bool,
    pub wire_frame_mode: bool,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneSettings {
    pub fn new() -> Self {
        Self {
            render_lights: false,
            wire_frame_mode: false,
        }
    }

    /// Toggle of we show the lights as cubes in the scene useful for debugging
    pub fn toggle_render_lights(&mut self) {
        self.render_lights = !self.render_lights;
    }

    /// Toggle showing wireframe models
    pub fn toggle_wire_frame_mode(&mut self) {
        self.wire_frame_mode = !self.wire_frame_mode;
    }
}
