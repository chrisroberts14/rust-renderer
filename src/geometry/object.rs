use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
}
