use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::scenes::material::Material;

#[allow(clippy::type_complexity)]
pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Material,
    update: Option<Box<dyn Fn(&mut Transform) + Send + Sync>>,
}

impl Object {
    pub fn new(mesh: Mesh, transform: Transform, material: Material) -> Self {
        Self {
            mesh,
            transform,
            material,
            update: None,
        }
    }

    /// Register a function that is called every update tick to animate this object.
    pub fn with_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Transform) + Send + Sync + 'static,
    {
        self.update = Some(Box::new(f));
        self
    }

    pub(crate) fn update(&mut self) {
        if let Some(f) = &self.update {
            f(&mut self.transform);
        }
    }
}
