pub mod pointlight;

use crate::maths::vec3::Vec3;

pub trait Light: Send + Sync {
    fn direction_to(&self, point: Vec3) -> Vec3;
    fn intensity_at(&self, point: Vec3) -> f32;
    fn colour_at(&self, point: Vec3) -> [f32; 3];
    fn position(&self) -> Vec3;
    fn colour(&self) -> [f32; 3];
}
