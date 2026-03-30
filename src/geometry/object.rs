use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::scenes::material::Material;

#[allow(clippy::type_complexity)]
pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Material,
    pub is_light: bool,
    update: Option<Box<dyn Fn(&mut Transform) + Send + Sync>>,
}

impl Clone for Object {
    /// Clones the object's geometry and material. The update closure is not cloned
    /// as closures aren't Clone — cloned objects are treated as static for rendering.
    fn clone(&self) -> Self {
        Self {
            mesh: self.mesh.clone(),
            transform: self.transform,
            material: self.material.clone(),
            is_light: self.is_light,
            update: None,
        }
    }
}

impl Object {
    pub fn new(mesh: Mesh, transform: Transform, material: Material) -> Self {
        Self {
            mesh,
            transform,
            material,
            is_light: false,
            update: None,
        }
    }

    /// Marks this object as a light source, causing it to be rendered unlit (full brightness).
    pub fn as_light(mut self) -> Self {
        self.is_light = true;
        self
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
