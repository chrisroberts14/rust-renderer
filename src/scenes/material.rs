use crate::scenes::texture::Texture;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Material {
    Color([u8; 4]),
    Texture(Arc<Texture>),
}
