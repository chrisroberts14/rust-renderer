use serde::Deserialize;
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

/// Parsed key bindings used at runtime to dispatch input events to actions
#[derive(Debug, Clone)]
pub struct KeyBindings {
    /// Maps character strings (e.g. "w", " ") to actions
    pub char_bindings: HashMap<String, Action>,
    /// Maps named key strings (e.g. "shift", "f1", "escape") to actions
    pub named_bindings: HashMap<String, Action>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindingsFile::default().into_key_bindings()
    }
}

impl KeyBindings {
    /// Load key bindings from a JSON file.
    ///
    /// Falls back to defaults if the file does not exist or cannot be parsed.
    pub fn from_file_or_default(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(data) => match serde_json::from_str::<KeyBindingsFile>(&data) {
                Ok(file) => file.into_key_bindings(),
                Err(e) => {
                    eprintln!("Failed to parse keybindings file '{path}': {e}, using defaults");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

/// JSON schema for the keybindings file.
///
/// Each field maps an action name to a key string. Character keys are written
/// as-is (e.g. `"w"`). The special value `"space"` binds the space bar.
/// Named keys are written in lowercase: `"shift"`, `"f1"`, `"escape"`.
#[derive(Deserialize)]
struct KeyBindingsFile {
    #[serde(default = "default_move_forward")]
    move_forward: String,
    #[serde(default = "default_move_backward")]
    move_backward: String,
    #[serde(default = "default_move_right")]
    move_right: String,
    #[serde(default = "default_move_left")]
    move_left: String,
    #[serde(default = "default_move_up")]
    move_up: String,
    #[serde(default = "default_move_down")]
    move_down: String,
    #[serde(default = "default_toggle_wireframe")]
    toggle_wireframe: String,
    #[serde(default = "default_toggle_lights")]
    toggle_lights: String,
    #[serde(default = "default_next_scene")]
    next_scene: String,
    #[serde(default = "default_increase_tiles")]
    increase_tiles: String,
    #[serde(default = "default_decrease_tiles")]
    decrease_tiles: String,
    #[serde(default = "default_toggle_overlay")]
    toggle_overlay: String,
    #[serde(default = "default_release_mouse")]
    release_mouse: String,
}

fn default_move_forward() -> String {
    "w".to_string()
}
fn default_move_backward() -> String {
    "s".to_string()
}
fn default_move_right() -> String {
    "d".to_string()
}
fn default_move_left() -> String {
    "a".to_string()
}
fn default_move_up() -> String {
    "space".to_string()
}
fn default_move_down() -> String {
    "shift".to_string()
}
fn default_toggle_wireframe() -> String {
    "m".to_string()
}
fn default_toggle_lights() -> String {
    "l".to_string()
}
fn default_next_scene() -> String {
    "n".to_string()
}
fn default_increase_tiles() -> String {
    "t".to_string()
}
fn default_decrease_tiles() -> String {
    "y".to_string()
}
fn default_toggle_overlay() -> String {
    "f1".to_string()
}
fn default_release_mouse() -> String {
    "escape".to_string()
}

impl Default for KeyBindingsFile {
    fn default() -> Self {
        Self {
            move_forward: default_move_forward(),
            move_backward: default_move_backward(),
            move_right: default_move_right(),
            move_left: default_move_left(),
            move_up: default_move_up(),
            move_down: default_move_down(),
            toggle_wireframe: default_toggle_wireframe(),
            toggle_lights: default_toggle_lights(),
            next_scene: default_next_scene(),
            increase_tiles: default_increase_tiles(),
            decrease_tiles: default_decrease_tiles(),
            toggle_overlay: default_toggle_overlay(),
            release_mouse: default_release_mouse(),
        }
    }
}

/// Keys that are treated as named (non-character) keys
const NAMED_KEYS: &[&str] = &["shift", "f1", "escape"];

impl KeyBindingsFile {
    fn into_key_bindings(self) -> KeyBindings {
        let pairs: Vec<(String, Action)> = vec![
            (self.move_forward, Action::MoveForward),
            (self.move_backward, Action::MoveBackward),
            (self.move_right, Action::MoveRight),
            (self.move_left, Action::MoveLeft),
            (self.move_up, Action::MoveUp),
            (self.move_down, Action::MoveDown),
            (self.toggle_wireframe, Action::ToggleWireframe),
            (self.toggle_lights, Action::ToggleLights),
            (self.next_scene, Action::NextScene),
            (self.increase_tiles, Action::IncreaseTiles),
            (self.decrease_tiles, Action::DecreaseTiles),
            (self.toggle_overlay, Action::ToggleOverlay),
            (self.release_mouse, Action::ReleaseMouse),
        ];

        let mut char_bindings = HashMap::new();
        let mut named_bindings = HashMap::new();

        for (key, action) in pairs {
            let normalized = key.to_lowercase();
            if NAMED_KEYS.contains(&normalized.as_str()) {
                named_bindings.insert(normalized, action);
            } else {
                // "space" is a user-friendly alias for the space character
                let ch = if normalized == "space" {
                    " ".to_string()
                } else {
                    normalized
                };
                char_bindings.insert(ch, action);
            }
        }

        KeyBindings {
            char_bindings,
            named_bindings,
        }
    }
}
