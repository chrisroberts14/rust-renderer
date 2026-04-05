use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
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

    /// Returns the world-space axis-aligned bounding box as (min, max), or None if the mesh has no vertices.
    pub(crate) fn bounding_box(&self) -> Option<(Vec3, Vec3)> {
        if self.mesh.vertices.is_empty() {
            return None;
        }
        let model = self.transform.matrix();
        let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &vertex in &self.mesh.vertices {
            let t = model * vertex.to_vec4();
            if t.x < min.x { min.x = t.x; }
            if t.y < min.y { min.y = t.y; }
            if t.z < min.z { min.z = t.z; }
            if t.x > max.x { max.x = t.x; }
            if t.y > max.y { max.y = t.y; }
            if t.z > max.z { max.z = t.z; }
        }
        Some((min, max))
    }

    /// Function to determine if a given point falls within the bounding box of the object
    pub(crate) fn is_within_bounding_box(&self, point: &Vec3) -> bool {
        let Some((min, max)) = self.bounding_box() else { return false };
        point.x >= min.x && point.x <= max.x
            && point.y >= min.y && point.y <= max.y
            && point.z >= min.z && point.z <= max.z
    }
}
