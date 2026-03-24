use schemars::JsonSchema;
use serde::Deserialize;

use crate::maths::vec3::Vec3;

#[derive(Deserialize, JsonSchema, Clone)]
pub struct PointLight {
    pub position: Vec3,
    pub colour: [f32; 3],
    pub intensity: f32,
}

impl PointLight {
    pub fn new(position: Vec3, colour: [f32; 3], intensity: f32) -> Self {
        Self {
            position,
            colour,
            intensity,
        }
    }

    pub fn direction_to(&self, point: Vec3) -> Vec3 {
        (self.position - point).normalise()
    }

    pub fn intensity_at(&self, point: Vec3) -> f32 {
        let diff = self.position - point;
        let distance_squared = diff.dot(diff);
        self.intensity / (1.0 + distance_squared)
    }

    pub fn colour_at(&self, point: Vec3) -> [f32; 3] {
        let intensity = self.intensity_at(point);
        [
            self.colour[0] * intensity,
            self.colour[1] * intensity,
            self.colour[2] * intensity,
        ]
    }
}
