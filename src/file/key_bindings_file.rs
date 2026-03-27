use std::collections::HashMap;
use std::fs;

/// All bindable actions in the renderer
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    ToggleWireframe,
    ToggleLights,
    NextScene,
    IncreaseTiles,
    DecreaseTiles,
    ToggleOverlay,
    ReleaseMouse,
}

fn action_from_str(s: &str) -> Option<Action> {
    match s {
        "move_forward" => Some(Action::MoveForward),
        "move_backward" => Some(Action::MoveBackward),
        "move_right" => Some(Action::MoveRight),
        "move_left" => Some(Action::MoveLeft),
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "toggle_wireframe" => Some(Action::ToggleWireframe),
        "toggle_lights" => Some(Action::ToggleLights),
        "next_scene" => Some(Action::NextScene),
        "increase_tiles" => Some(Action::IncreaseTiles),
        "decrease_tiles" => Some(Action::DecreaseTiles),
        "toggle_overlay" => Some(Action::ToggleOverlay),
        "release_mouse" => Some(Action::ReleaseMouse),
        _ => None,
    }
}

/// Default action → key mappings used when a key is absent from the file.
/// Character keys are written as-is; `"space"` is an alias for the space bar.
/// Named keys are lowercase strings: `"shift"`, `"f1"`, `"escape"`.
const DEFAULTS: &[(&str, &str)] = &[
    ("move_forward", "w"),
    ("move_backward", "s"),
    ("move_right", "d"),
    ("move_left", "a"),
    ("move_up", "space"),
    ("move_down", "shift"),
    ("toggle_wireframe", "m"),
    ("toggle_lights", "l"),
    ("next_scene", "n"),
    ("increase_tiles", "t"),
    ("decrease_tiles", "y"),
    ("toggle_overlay", "f1"),
    ("release_mouse", "escape"),
];

/// Runtime key bindings mapping key strings to actions.
///
/// Character keys are stored as-is (space bar as `" "`).
/// Named keys are stored as lowercase strings (e.g. `"shift"`, `"f1"`, `"escape"`).
#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub bindings: HashMap<String, Action>,
}

impl KeyBindings {
    /// Load key bindings from a JSON file.
    ///
    /// Falls back to defaults for any missing keys; silently uses all defaults
    /// if the file does not exist or cannot be parsed.
    pub fn from_file_or_default(path: &str) -> Self {
        let file_map: HashMap<String, String> = fs::read_to_string(path)
            .ok()
            .and_then(|data| {
                serde_json::from_str(&data)
                    .map_err(|e| eprintln!("Failed to parse keybindings file '{path}': {e}"))
                    .ok()
            })
            .unwrap_or_default();

        let bindings = DEFAULTS
            .iter()
            .filter_map(|(action_name, default_key)| {
                let key = file_map
                    .get(*action_name)
                    .map(String::as_str)
                    .unwrap_or(default_key);
                let normalized = key.to_lowercase();
                let ch = if normalized == "space" {
                    " ".to_string()
                } else {
                    normalized
                };
                Some((ch, action_from_str(action_name)?))
            })
            .collect();

        KeyBindings { bindings }
    }
}
