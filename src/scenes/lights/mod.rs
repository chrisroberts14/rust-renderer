pub mod pointlight;
pub mod spot_light;

use crate::maths::vec3::Vec3;

pub trait Light: Send + Sync + std::fmt::Debug {
    fn direction_to(&self, point: Vec3) -> Vec3;
    fn intensity_at(&self, point: Vec3) -> f32;
    fn colour_at(&self, point: Vec3) -> [f32; 3];
    fn position(&self) -> Vec3;
    fn colour(&self) -> [f32; 3];
    fn intensity(&self) -> f32;
    /// Spot light cone direction. Returns `None` for point lights.
    fn spot_direction(&self) -> Option<Vec3> {
        None
    }
    /// Half-angle of the full cone in radians. `0.0` for point lights.
    fn cone_angle(&self) -> f32 {
        0.0
    }
    /// Width of the falloff band at the cone edge in radians. `0.0` for point lights.
    fn falloff_angle(&self) -> f32 {
        0.0
    }
}
