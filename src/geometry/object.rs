use crate::geometry::animation::Animation;
use crate::geometry::mesh::Mesh;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::material::Material;
use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

pub struct Object {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Material,
    pub is_light: bool,
    animation: Option<Arc<dyn Animation>>,
}

impl Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Object")
            .field("mesh", &self.mesh)
            .field("transform", &self.transform)
            .field("material", &self.material)
            .field("is_light", &self.is_light)
            .field("animation", &self.animation.as_ref().map(|_| "<animation>"))
            .finish()
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        Self {
            mesh: self.mesh.clone(),
            transform: self.transform,
            material: self.material.clone(),
            is_light: self.is_light,
            animation: self.animation.clone(),
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
            animation: None,
        }
    }

    /// Marks this object as a light source, causing it to be rendered unlit (full brightness).
    pub fn as_light(mut self) -> Self {
        self.is_light = true;
        self
    }

    /// Attach an animation that is called every update tick.
    pub fn with_animation(mut self, animation: impl Animation + 'static) -> Self {
        self.animation = Some(Arc::new(animation));
        self
    }

    /// Returns the attached animation, if any.
    pub fn animation(&self) -> Option<&dyn Animation> {
        self.animation.as_deref()
    }

    pub(crate) fn update(&mut self) {
        if let Some(anim) = &self.animation {
            anim.tick(&mut self.transform);
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
