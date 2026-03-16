use crate::maths::vec3::Vec3;

#[allow(dead_code)]
pub struct PointLight {
    pub position: Vec3,
    pub colour: [f32; 3],
    pub intensity: f32,
}

#[allow(dead_code)]
impl PointLight {
    pub fn new(position: Vec3, colour: [f32; 3], intensity: f32) -> Self {
        Self {
            position,
            colour,
            intensity,
        }
    }

    pub fn direction_to(&self, point: Vec3) -> Vec3 {
        Vec3 {
            x: self.position.x - point.x,
            y: self.position.y - point.y,
            z: self.position.z - point.z,
        }
        .normalise()
    }

    pub fn intensity_at(&self, point: Vec3) -> f32 {
        let distance_squared = (self.position.x - point.x).powi(2)
            + (self.position.y - point.y).powi(2)
            + (self.position.z - point.z).powi(2);
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
