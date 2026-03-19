use std::sync::Arc;

use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::texture::Texture;

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub texture: Option<Arc<Texture>>,
}

impl Object {
    pub fn new(mesh: Mesh, transform: Transform) -> Self {
        Self {
            mesh,
            transform,
            texture: None,
        }
    }
}
