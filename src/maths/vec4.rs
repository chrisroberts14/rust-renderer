use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

use crate::maths::vec3::Vec3;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn perspective_divide(self) -> Result<Vec3, &'static str> {
        if self.w == 0.0 {
            return Err("Cannot divide by zero (w is 0)");
        }
        Ok(Vec3 {
            x: self.x / self.w,
            y: self.y / self.w,
            z: self.z / self.w,
        })
    }

    pub fn from_vec3(v: Vec3, w: f32) -> Vec4 {
        Vec4 {
            x: v.x,
            y: v.y,
            z: v.z,
            w,
        }
    }

    pub fn to_vec3(self) -> Vec3 {
        Vec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

impl Sub for Vec4 {
    type Output = Vec4;

    fn sub(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl Add for Vec4 {
    type Output = Vec4;

    fn add(self, other: Vec4) -> Vec4 {
        Vec4 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Vec4;

    fn mul(self, scalar: f32) -> Vec4 {
        Vec4 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(
            Vec4::new(1.0, 2.0, 3.0, 4.0) + Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(6.0, 8.0, 10.0, 12.0)
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            Vec4::new(5.0, 6.0, 7.0, 8.0) - Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(4.0, 4.0, 4.0, 4.0)
        );
    }

    #[test]
    fn test_mul_scalar() {
        assert_eq!(
            Vec4::new(1.0, 2.0, 3.0, 4.0) * 2.0,
            Vec4::new(2.0, 4.0, 6.0, 8.0)
        );
    }

    #[test]
    fn test_from_vec3() {
        let v3 = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(Vec4::from_vec3(v3, 5.0), Vec4::new(1.0, 2.0, 3.0, 5.0));
    }

    #[test]
    fn test_to_vec3_drops_w() {
        assert_eq!(
            Vec4::new(1.0, 2.0, 3.0, 99.0).to_vec3(),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn test_perspective_divide() {
        let result = Vec4::new(2.0, 4.0, 6.0, 2.0).perspective_divide().unwrap();
        assert_eq!(result, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_perspective_divide_zero_w_is_err() {
        assert!(Vec4::new(1.0, 2.0, 3.0, 0.0).perspective_divide().is_err());
    }
}
