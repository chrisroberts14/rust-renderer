use schemars::JsonSchema;
use serde::Deserialize;
use std::ops::Mul;

use crate::maths::{mat4::Mat4, vec3::Vec3};

fn default_position() -> Vec3 {
    Vec3::ZERO
}

fn default_rotation() -> Vec3 {
    Vec3::ZERO
}

fn default_scale() -> Vec3 {
    Vec3::ONE
}

#[derive(JsonSchema, Deserialize, Clone, Copy)]
pub struct Transform {
    #[serde(default = "default_position")]
    pub position: Vec3,
    #[serde(default = "default_rotation")]
    pub rotation: Vec3,
    #[serde(default = "default_scale")]
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn in_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }

    pub fn matrix(&self) -> Mat4 {
        let translation = Mat4::translation(self.position.x, self.position.y, self.position.z);

        let rx = Mat4::rotation_x(self.rotation.x);
        let ry = Mat4::rotation_y(self.rotation.y);
        let rz = Mat4::rotation_z(self.rotation.z);

        let scale = Mat4::scale(self.scale.x, self.scale.y, self.scale.z);

        translation * rz * ry * rx * scale
    }

    /// Returns the model matrix and the normal matrix (inverse-transpose of model).
    /// Used to correctly transform normals when the model has non-uniform scaling.
    pub fn matrices(&self) -> (Mat4, Mat4) {
        let model = self.matrix();
        // Degenerate transforms (such as scale 0.0) should not panic the renderer
        if let Some(normal) = model.inverse() {
            let normal = normal.transpose();
            return (model, normal);
        }
        (model, Mat4::identity())
    }
}

impl Mul for Transform {
    type Output = Transform;

    /// Compose two transforms: `self` is applied first, then `rhs`.
    /// Equivalent to multiplying their model matrices and decomposing back to TRS.
    fn mul(self, rhs: Transform) -> Transform {
        let m = rhs.matrix() * self.matrix();
        let mat = m.m;

        let tx = mat[0][3];
        let ty = mat[1][3];
        let tz = mat[2][3];

        let sx = (mat[0][0] * mat[0][0] + mat[1][0] * mat[1][0] + mat[2][0] * mat[2][0]).sqrt();
        let sy = (mat[0][1] * mat[0][1] + mat[1][1] * mat[1][1] + mat[2][1] * mat[2][1]).sqrt();
        let sz = (mat[0][2] * mat[0][2] + mat[1][2] * mat[1][2] + mat[2][2] * mat[2][2]).sqrt();

        let ry = (-(mat[2][0] / sx)).asin();
        let rx = (mat[2][1] / sy).atan2(mat[2][2] / sz);
        let rz = (mat[1][0] / sx).atan2(mat[0][0] / sx);

        Transform {
            position: Vec3::new(tx, ty, tz),
            rotation: Vec3::new(rx, ry, rz),
            scale: Vec3::new(sx, sy, sz),
        }
    }
}
