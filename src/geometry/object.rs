use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::material::Material;

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Material,
}

impl Object {
    pub fn new(mesh: Mesh, transform: Transform, material: Material) -> Self {
        Self {
            mesh,
            transform,
            material,
        }
    }
}
