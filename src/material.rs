use crate::texture::Texture;
use std::sync::Arc;

#[allow(dead_code)]
pub enum Material {
    Color([u8; 4]),
    Texture(Arc<Texture>),
}
