pub struct Tile {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Tile {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Divides the framebuffer into a grid of tiles of `tile_size` pixels.
/// Edge tiles are smaller if the dimensions don't divide evenly.
pub fn make_tiles(fb_width: usize, fb_height: usize, tile_size: usize) -> Vec<Tile> {
    let cols = fb_width.div_ceil(tile_size);
    let rows = fb_height.div_ceil(tile_size);
    let mut tiles = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let x = col * tile_size;
            let y = row * tile_size;
            tiles.push(Tile::new(
                x,
                y,
                tile_size.min(fb_width - x),
                tile_size.min(fb_height - y),
            ));
        }
    }
    tiles
}
