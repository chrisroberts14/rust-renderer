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
    SpeedModifier,
    ToggleWireframe,
    ToggleLights,
    NextScene,
    IncreaseTiles,
    DecreaseTiles,
    ToggleOverlay,
    ReleaseMouse,
    NextRenderer,
}

fn action_from_str(s: &str) -> Option<Action> {
    match s {
        "move_forward" => Some(Action::MoveForward),
        "move_backward" => Some(Action::MoveBackward),
        "move_right" => Some(Action::MoveRight),
        "move_left" => Some(Action::MoveLeft),
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "speed_modifier" => Some(Action::SpeedModifier),
        "toggle_wireframe" => Some(Action::ToggleWireframe),
        "toggle_lights" => Some(Action::ToggleLights),
        "next_scene" => Some(Action::NextScene),
        "increase_tiles" => Some(Action::IncreaseTiles),
        "decrease_tiles" => Some(Action::DecreaseTiles),
        "toggle_overlay" => Some(Action::ToggleOverlay),
        "release_mouse" => Some(Action::ReleaseMouse),
        "next_renderer" => Some(Action::NextRenderer),
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
    ("speed_modifier", "ctrl"),
    ("toggle_wireframe", "m"),
    ("toggle_lights", "l"),
    ("next_scene", "n"),
    ("increase_tiles", "t"),
    ("decrease_tiles", "y"),
    ("toggle_overlay", "f1"),
    ("release_mouse", "escape"),
    ("next_renderer", "r"),
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

        Self::from_map(file_map)
    }

    fn from_map(file_map: HashMap<String, String>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> KeyBindings {
        KeyBindings::from_map(HashMap::new())
    }

    #[test]
    fn all_actions_are_bound_by_default() {
        assert_eq!(defaults().bindings.len(), DEFAULTS.len());
    }

    #[test]
    fn file_overrides_default_key() {
        let kb = KeyBindings::from_map(HashMap::from([(
            "move_forward".to_string(),
            "i".to_string(),
        )]));
        assert_eq!(kb.bindings.get("i"), Some(&Action::MoveForward));
        assert_eq!(kb.bindings.get("w"), None);
    }

    #[test]
    fn aliases_are_converted() {
        let kb = KeyBindings::from_map(HashMap::from([(
            "move_up".to_string(),
            "space".to_string(),
        )]));
        assert_eq!(kb.bindings.get(" "), Some(&Action::MoveUp));
        assert_eq!(kb.bindings.get("space"), None);
    }

    #[test]
    fn keys_are_normalised_to_lowercase() {
        let kb = KeyBindings::from_map(HashMap::from([(
            "move_forward".to_string(),
            "W".to_string(),
        )]));
        assert_eq!(kb.bindings.get("w"), Some(&Action::MoveForward));
        assert_eq!(kb.bindings.get("W"), None);
    }

    #[test]
    fn unknown_action_in_file_is_ignored() {
        let kb = KeyBindings::from_map(HashMap::from([(
            "nonexistent_action".to_string(),
            "z".to_string(),
        )]));
        assert_eq!(kb.bindings.get("z"), None);
        assert_eq!(kb.bindings.len(), DEFAULTS.len());
    }
}
