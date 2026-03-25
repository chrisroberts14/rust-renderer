use crate::maths::vec3::Vec3;
use crate::scenes::lights::Light;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema, Clone)]
pub struct SpotLight {
    position: Vec3,
    direction: Vec3,
    colour: [f32; 3],
    intensity: f32,
    cone_angle: f32,
    falloff_angle: f32,
}

impl SpotLight {
    pub fn new(
        position: Vec3,
        direction: Vec3,
        colour: [f32; 3],
        intensity: f32,
        cone_angle: f32,
        falloff_angle: f32,
    ) -> SpotLight {
        SpotLight {
            position,
            direction: direction.normalise(),
            colour,
            intensity,
            cone_angle,
            falloff_angle,
        }
    }
}

impl Light for SpotLight {
    fn direction_to(&self, point: Vec3) -> Vec3 {
        (self.position - point).normalise()
    }

    fn intensity_at(&self, point: Vec3) -> f32 {
        let diff = self.position - point;
        let distance_squared = diff.dot(diff);
        let distance_attenuation = self.intensity / (1.0 + distance_squared);

        let to_point = (point - self.position).normalise();
        let angle_to_point = self.direction.dot(to_point).acos();

        if angle_to_point > self.cone_angle {
            return 0.0;
        }

        let inner_angle = self.cone_angle - self.falloff_angle;
        let cone_attenuation = if angle_to_point <= inner_angle {
            1.0
        } else {
            // Smoothstep blend from 1 to 0 across the falloff band
            let t = (angle_to_point - inner_angle) / self.falloff_angle;
            1.0 - t * t * (3.0 - 2.0 * t)
        };

        distance_attenuation * cone_attenuation
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
}
