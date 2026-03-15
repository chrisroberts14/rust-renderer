use crate::maths::vec2::Vec2;
use std::ops::Add;
use std::ops::Sub;

#[derive(Copy, Clone, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[allow(dead_code)]
impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn scale(&self, factor: f32) -> Vec3 {
        Vec3::new(self.x * factor, self.y * factor, self.z * factor)
    }

    pub fn project_to_2d(&self, width: usize, height: usize) -> Vec2 {
        let x = ((self.x + 1.0) * 0.5 * width as f32) as usize;
        let y = ((1.0 - (self.y + 1.0) * 0.5) * height as f32) as usize;
        Vec2::new(x as f32, y as f32)
    }

    // Rotate around X axis
    pub fn rotate_x(&self, angle_rad: f32) -> Vec3 {
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        Vec3 {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    // Rotate around Y axis
    pub fn rotate_y(&self, angle_rad: f32) -> Vec3 {
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        Vec3 {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    // Rotate around Z axis
    pub fn rotate_z(&self, angle_rad: f32) -> Vec3 {
        let cos = angle_rad.cos();
        let sin = angle_rad.sin();
        Vec3 {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}
