use schemars::JsonSchema;
use serde::Deserialize;

use crate::maths::vec3::Vec3;
use crate::scenes::lights::Light;

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
}

impl Light for PointLight {
    fn direction_to(&self, point: Vec3) -> Vec3 {
        (self.position - point).normalise()
    }

    fn intensity_at(&self, point: Vec3) -> f32 {
        let diff = self.position - point;
        let distance_squared = diff.dot(diff);
        self.intensity / (1.0 + distance_squared)
    }

    fn colour_at(&self, point: Vec3) -> [f32; 3] {
        let intensity = self.intensity_at(point);
        [
            self.colour[0] * intensity,
            self.colour[1] * intensity,
            self.colour[2] * intensity,
        ]
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn colour(&self) -> [f32; 3] {
        self.colour
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }
}
