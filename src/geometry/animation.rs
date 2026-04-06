use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;

/// Defines how an object's transform is updated each tick.
///
/// Implement this trait to create testable, inspectable animations. Unlike a bare closure,
/// an `Animation` impl can be constructed with known parameters and asserted against in tests.
pub trait Animation: Send + Sync {
    fn tick(&self, transform: &mut Transform);
}

/// Applies a fixed rotation and position delta to the transform every tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeltaAnimation {
    pub rotation: Vec3,
    pub position: Vec3,
}

impl Animation for DeltaAnimation {
    fn tick(&self, transform: &mut Transform) {
        transform.rotation = transform.rotation + self.rotation;
        transform.position = transform.position + self.position;
    }
}
