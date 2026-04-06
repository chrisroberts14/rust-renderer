use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::material::Material;
use std::fmt;
use std::fmt::Debug;

#[allow(clippy::type_complexity)]
pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Material,
    pub is_light: bool,
    update: Option<Box<dyn Fn(&mut Transform) + Send + Sync>>,
}

impl Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Object")
            .field("mesh", &self.mesh)
            .field("transform", &self.transform)
            .field("material", &self.material)
            .field("is_light", &self.is_light)
            .field("update", &"<closure>")
            .finish()
    }
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
        let (min, max) = self.mesh.aabb_bounding_box?;

        let model = self.transform.matrix();

        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
        ];

        let mut new_min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut new_max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for corner in corners {
            let t = (model * corner.to_vec4()).to_vec3();
            new_min = new_min.min(t);
            new_max = new_max.max(t);
        }

        Some((new_min, new_max))
    }

    /// Function to determine if a given point falls within the bounding box of the object
    pub(crate) fn is_within_bounding_box(&self, point: &Vec3) -> bool {
        let Some((min, max)) = self.bounding_box() else {
            return false;
        };
        point.x >= min.x
            && point.x <= max.x
            && point.y >= min.y
            && point.y <= max.y
            && point.z >= min.z
            && point.z <= max.z
    }
}
