use schemars::JsonSchema;
use serde::Deserialize;

use crate::maths::vec2::Vec2;
use crate::maths::vec4::Vec4;
use std::ops::Add;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Neg;
use std::ops::Sub;

#[derive(Copy, Clone, Debug, JsonSchema, Deserialize, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn scale(&self, factor: f32) -> Vec3 {
        Vec3::new(self.x * factor, self.y * factor, self.z * factor)
    }

    pub fn dot(&self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length(&self) -> f32 {
        f32::sqrt(self.x * self.x + self.y * self.y + self.z * self.z)
    }

    pub fn normalise(&self) -> Vec3 {
        let len = self.length();
        Vec3 {
            x: self.x / len,
            y: self.y / len,
            z: self.z / len,
        }
    }

    pub fn project_to_2d(&self, width: usize, height: usize) -> Vec2 {
        let x = ((self.x + 1.0) * 0.5 * (width - 1) as f32) as usize;
        let y = ((1.0 - (self.y + 1.0) * 0.5) * (height - 1) as f32) as usize;
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

    pub fn to_vec4(self) -> Vec4 {
        Vec4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w: 1.0,
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

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, factor: f32) -> Vec3 {
        Vec3 {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(self, factor: f32) -> Vec3 {
        Vec3 {
            x: self.x / factor,
            y: self.y / factor,
            z: self.z / factor,
        }
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec3, b: Vec3) -> bool {
        (a.x - b.x).abs() < 1e-5 && (a.y - b.y).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5
    }

    #[test]
    fn test_add() {
        assert_eq!(
            Vec3::new(1.0, 2.0, 3.0) + Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(5.0, 7.0, 9.0)
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            Vec3::new(5.0, 7.0, 9.0) - Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0)
        );
    }

    #[test]
    fn test_mul_scalar() {
        assert_eq!(Vec3::new(1.0, 2.0, 3.0) * 3.0, Vec3::new(3.0, 6.0, 9.0));
    }

    #[test]
    fn test_div_scalar() {
        assert_eq!(Vec3::new(3.0, 6.0, 9.0) / 3.0, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Vec3::new(1.0, -2.0, 3.0), Vec3::new(-1.0, 2.0, -3.0));
    }

    #[test]
    fn test_scale() {
        assert_eq!(
            Vec3::new(1.0, 2.0, 3.0).scale(2.0),
            Vec3::new(2.0, 4.0, 6.0)
        );
    }

    #[test]
    fn test_dot_perpendicular() {
        assert_eq!(Vec3::new(1.0, 0.0, 0.0).dot(Vec3::new(0.0, 1.0, 0.0)), 0.0);
    }

    #[test]
    fn test_dot_value() {
        // (1,2,3)·(4,5,6) = 4+10+18 = 32
        assert_eq!(Vec3::new(1.0, 2.0, 3.0).dot(Vec3::new(4.0, 5.0, 6.0)), 32.0);
    }

    #[test]
    fn test_dot_self_equals_length_squared() {
        let v = Vec3::new(2.0, 3.0, 6.0);
        assert_eq!(v.dot(v), v.length() * v.length());
    }

    #[test]
    fn test_cross_basis_vectors() {
        // x × y = z, y × z = x, z × x = y
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(x.cross(y), z);
        assert_eq!(y.cross(z), x);
        assert_eq!(z.cross(x), y);
    }

    #[test]
    fn test_cross_self_is_zero() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.cross(v), Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_length() {
        // (2, 3, 6): sqrt(4+9+36) = sqrt(49) = 7
        assert_eq!(Vec3::new(2.0, 3.0, 6.0).length(), 7.0);
    }

    #[test]
    fn test_normalise() {
        let n = Vec3::new(2.0, 3.0, 6.0).normalise(); // length 7
        assert!(
            approx(n, Vec3::new(2.0 / 7.0, 3.0 / 7.0, 6.0 / 7.0)),
            "{n:?}"
        );
        assert!((n.length() - 1.0).abs() < 1e-5, "length = {}", n.length());
    }

    #[test]
    fn test_rotate_x() {
        // (0, 1, 0) rotated 90° around X → (0, 0, 1)
        let r = Vec3::new(0.0, 1.0, 0.0).rotate_x(std::f32::consts::FRAC_PI_2);
        assert!(approx(r, Vec3::new(0.0, 0.0, 1.0)), "{r:?}");
    }

    #[test]
    fn test_rotate_y() {
        // (1, 0, 0) rotated 90° around Y → (0, 0, -1)
        let r = Vec3::new(1.0, 0.0, 0.0).rotate_y(std::f32::consts::FRAC_PI_2);
        assert!(approx(r, Vec3::new(0.0, 0.0, -1.0)), "{r:?}");
    }

    #[test]
    fn test_rotate_z() {
        // (1, 0, 0) rotated 90° around Z → (0, 1, 0)
        let r = Vec3::new(1.0, 0.0, 0.0).rotate_z(std::f32::consts::FRAC_PI_2);
        assert!(approx(r, Vec3::new(0.0, 1.0, 0.0)), "{r:?}");
    }

    #[test]
    fn test_to_vec4() {
        let v4 = Vec3::new(1.0, 2.0, 3.0).to_vec4();
        assert_eq!(v4, Vec4::new(1.0, 2.0, 3.0, 1.0));
    }
}
