# rust-renderer

A 3D renderer written in Rust with four interchangeable backends: two software rasterizers (single and multi-threaded) and two GPU renderers (wgpu/WGSL and Vulkan).

## Running

```bash
cargo run --release
```

The WGSL renderer is used by default. Available CLI flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--renderer` | `wgsl` | Renderer backend (see [Renderers](#renderers)) |
| `--width` | `800` | Window width in pixels |
| `--height` | `600` | Window height in pixels |
| `--scene` | *(none)* | Path to a scene JSON file; cycles through `assets/scene_defs/` if omitted |

## Renderers

| Name | Description |
|------|-------------|
| `single-thread-cpu` | Tile-based software rasterizer, single-threaded |
| `multi-thread-cpu` | Tile-based software rasterizer, parallel tiles via Rayon |
| `wgsl` *(default)* | GPU renderer using wgpu with WGSL shaders |
| `vulkan` | GPU renderer using Vulkano |

Select a renderer at startup:

```bash
cargo run --release -- --renderer vulkan
```

Press `R` while running to cycle through all renderers without restarting.

## Scene Files

Scenes are defined in JSON. By default the app cycles through all `.json` files in `assets/scene_defs/`. Pass `--scene <path>` to load a specific file. Scene files support **live hot-reload** — save the file and the renderer picks up changes automatically.

### Object types

#### `mesh` — OBJ file

```json
{
  "type": "mesh",
  "obj_path": "assets/monkey.obj",
  "transform": { "position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1] },
  "colour": [255, 255, 255, 255],
  "update": { "position": [0, 0, 0], "rotation": [0.01, 0.01, 0], "scale": [0, 0, 0] }
}
```

`colour` defaults to white. `update` is optional and drives per-frame animation — the `position` and `rotation` values are added as deltas each frame.

#### `plane` — flat quad

```json
{
  "type": "plane",
  "size": 40,
  "subdivisions": 8,
  "transform": { "position": [0, -2, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1] },
  "colour": [255, 255, 255, 255]
}
```

`subdivisions` defaults to `8`.

#### `sphere`

```json
{
  "type": "sphere",
  "radius": 1.0,
  "stacks": 16,
  "slices": 16,
  "transform": { "position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1] },
  "colour": [255, 255, 255, 255]
}
```

`stacks` and `slices` both default to `16`.

### Light types

#### `point`

```json
{ "type": "point", "position": [0, 5, 0], "colour": [1.0, 1.0, 1.0], "intensity": 100 }
```

#### `spot`

```json
{ "type": "spot", "position": [0, 5, 0], "direction": [0, -1, 0], "colour": [1.0, 1.0, 1.0], "intensity": 100, "cutoff": 0.5 }
```

### Skybox

```json
{ "type": "file", "path": "assets/skybox.exr" }
```

```json
{ "type": "solid_colour", "colour": [30, 30, 30, 255] }
```

### Top-level fields

```json
{
  "ambient": 0.15,
  "skybox": { ... },
  "objects": [ ... ],
  "lights": [ ... ]
}
```

`ambient` defaults to `0.15`.

## Controls

Default keybindings — customisable via `assets/keybindings.json` (see below).

| Key | Action |
|-----|--------|
| `W` / `S` | Move forward / backward |
| `A` / `D` | Move left / right |
| `Space` / `Shift` | Move up / down |
| `Ctrl` | Speed modifier (hold for faster movement) |
| `M` | Toggle wireframe mode |
| `L` | Toggle light source visibility |
| `N` | Next scene |
| `R` | Cycle to next renderer |
| `T` / `Y` | Increase / decrease tile count (CPU renderers only) |
| `Escape` | Release mouse cursor |

### Custom keybindings

Create or edit `assets/keybindings.json` with action-to-key mappings:

```json
{
  "move_forward": "i",
  "move_backward": "k"
}
```

Any action omitted from the file falls back to its default. Available actions:
`move_forward`, `move_backward`, `move_left`, `move_right`, `move_up`, `move_down`,
`speed_modifier`, `toggle_wireframe`, `toggle_lights`, `next_scene`, `next_renderer`,
`increase_tiles`, `decrease_tiles`, `release_mouse`.

## Benchmarks

```bash
cargo bench --bench renderer
cargo bench --bench geometry
cargo bench --bench framebuffer
```