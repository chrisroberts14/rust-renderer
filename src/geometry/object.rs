use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
}

impl Object {
    pub fn new(mesh: Mesh, transform: Transform) -> Self {
        Self { mesh, transform }
    }
}
