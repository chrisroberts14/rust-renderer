use std::array;
use std::ops::Mul;

use crate::maths::vec4::Vec4;

#[derive(Copy, Clone, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, x],
                [0.0, 1.0, 0.0, y],
                [0.0, 0.0, 1.0, z],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_x(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();

        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, -s, 0.0],
                [0.0, s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_y(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();

        Self {
            m: [
                [c, 0.0, s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [-s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn rotation_z(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();

        Self {
            m: [
                [c, -s, 0.0, 0.0],
                [s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov * 0.5).tan();
        let nf = 1.0 / (near - far);

        Self {
            m: [
                [f / aspect, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (far + near) * nf, (2.0 * far * near) * nf],
                [0.0, 0.0, -1.0, 0.0],
            ],
        }
    }

    pub fn transpose(&self) -> Mat4 {
        Mat4 {
            m: array::from_fn(|i| array::from_fn(|j| self.m[j][i])),
        }
    }

    pub fn inverse(&self) -> Option<Mat4> {
        let m = &self.m;

        let mut inv = [[0.0f32; 4]; 4];

        inv[0][0] =
            m[1][1] * m[2][2] * m[3][3] - m[1][1] * m[2][3] * m[3][2] - m[2][1] * m[1][2] * m[3][3]
                + m[2][1] * m[1][3] * m[3][2]
                + m[3][1] * m[1][2] * m[2][3]
                - m[3][1] * m[1][3] * m[2][2];

        inv[0][1] = -m[0][1] * m[2][2] * m[3][3]
            + m[0][1] * m[2][3] * m[3][2]
            + m[2][1] * m[0][2] * m[3][3]
            - m[2][1] * m[0][3] * m[3][2]
            - m[3][1] * m[0][2] * m[2][3]
            + m[3][1] * m[0][3] * m[2][2];

        inv[0][2] =
            m[0][1] * m[1][2] * m[3][3] - m[0][1] * m[1][3] * m[3][2] - m[1][1] * m[0][2] * m[3][3]
                + m[1][1] * m[0][3] * m[3][2]
                + m[3][1] * m[0][2] * m[1][3]
                - m[3][1] * m[0][3] * m[1][2];

        inv[0][3] = -m[0][1] * m[1][2] * m[2][3]
            + m[0][1] * m[1][3] * m[2][2]
            + m[1][1] * m[0][2] * m[2][3]
            - m[1][1] * m[0][3] * m[2][2]
            - m[2][1] * m[0][2] * m[1][3]
            + m[2][1] * m[0][3] * m[1][2];

        inv[1][0] = -m[1][0] * m[2][2] * m[3][3]
            + m[1][0] * m[2][3] * m[3][2]
            + m[2][0] * m[1][2] * m[3][3]
            - m[2][0] * m[1][3] * m[3][2]
            - m[3][0] * m[1][2] * m[2][3]
            + m[3][0] * m[1][3] * m[2][2];

        inv[1][1] =
            m[0][0] * m[2][2] * m[3][3] - m[0][0] * m[2][3] * m[3][2] - m[2][0] * m[0][2] * m[3][3]
                + m[2][0] * m[0][3] * m[3][2]
                + m[3][0] * m[0][2] * m[2][3]
                - m[3][0] * m[0][3] * m[2][2];

        inv[1][2] = -m[0][0] * m[1][2] * m[3][3]
            + m[0][0] * m[1][3] * m[3][2]
            + m[1][0] * m[0][2] * m[3][3]
            - m[1][0] * m[0][3] * m[3][2]
            - m[3][0] * m[0][2] * m[1][3]
            + m[3][0] * m[0][3] * m[1][2];

        inv[1][3] =
            m[0][0] * m[1][2] * m[2][3] - m[0][0] * m[1][3] * m[2][2] - m[1][0] * m[0][2] * m[2][3]
                + m[1][0] * m[0][3] * m[2][2]
                + m[2][0] * m[0][2] * m[1][3]
                - m[2][0] * m[0][3] * m[1][2];

        inv[2][0] =
            m[1][0] * m[2][1] * m[3][3] - m[1][0] * m[2][3] * m[3][1] - m[2][0] * m[1][1] * m[3][3]
                + m[2][0] * m[1][3] * m[3][1]
                + m[3][0] * m[1][1] * m[2][3]
                - m[3][0] * m[1][3] * m[2][1];

        inv[2][1] = -m[0][0] * m[2][1] * m[3][3]
            + m[0][0] * m[2][3] * m[3][1]
            + m[2][0] * m[0][1] * m[3][3]
            - m[2][0] * m[0][3] * m[3][1]
            - m[3][0] * m[0][1] * m[2][3]
            + m[3][0] * m[0][3] * m[2][1];

        inv[2][2] =
            m[0][0] * m[1][1] * m[3][3] - m[0][0] * m[1][3] * m[3][1] - m[1][0] * m[0][1] * m[3][3]
                + m[1][0] * m[0][3] * m[3][1]
                + m[3][0] * m[0][1] * m[1][3]
                - m[3][0] * m[0][3] * m[1][1];

        inv[2][3] = -m[0][0] * m[1][1] * m[2][3]
            + m[0][0] * m[1][3] * m[2][1]
            + m[1][0] * m[0][1] * m[2][3]
            - m[1][0] * m[0][3] * m[2][1]
            - m[2][0] * m[0][1] * m[1][3]
            + m[2][0] * m[0][3] * m[1][1];

        inv[3][0] = -m[1][0] * m[2][1] * m[3][2]
            + m[1][0] * m[2][2] * m[3][1]
            + m[2][0] * m[1][1] * m[3][2]
            - m[2][0] * m[1][2] * m[3][1]
            - m[3][0] * m[1][1] * m[2][2]
            + m[3][0] * m[1][2] * m[2][1];

        inv[3][1] =
            m[0][0] * m[2][1] * m[3][2] - m[0][0] * m[2][2] * m[3][1] - m[2][0] * m[0][1] * m[3][2]
                + m[2][0] * m[0][2] * m[3][1]
                + m[3][0] * m[0][1] * m[2][2]
                - m[3][0] * m[0][2] * m[2][1];

        inv[3][2] = -m[0][0] * m[1][1] * m[3][2]
            + m[0][0] * m[1][2] * m[3][1]
            + m[1][0] * m[0][1] * m[3][2]
            - m[1][0] * m[0][2] * m[3][1]
            - m[3][0] * m[0][1] * m[1][2]
            + m[3][0] * m[0][2] * m[1][1];

        inv[3][3] =
            m[0][0] * m[1][1] * m[2][2] - m[0][0] * m[1][2] * m[2][1] - m[1][0] * m[0][1] * m[2][2]
                + m[1][0] * m[0][2] * m[2][1]
                + m[2][0] * m[0][1] * m[1][2]
                - m[2][0] * m[0][2] * m[1][1];

        let det =
            m[0][0] * inv[0][0] + m[0][1] * inv[1][0] + m[0][2] * inv[2][0] + m[0][3] * inv[3][0];

        if det == 0.0 {
            return None;
        }

        let det_inv = 1.0 / det;

        inv.iter_mut()
            .for_each(|row| row.iter_mut().for_each(|x| *x *= det_inv));

        Some(Mat4 { m: inv })
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Mat4 {
        let mut result = [[0.0; 4]; 4];

        for (row_index, row) in self.m.iter().enumerate() {
            for (col_index, _) in row.iter().enumerate() {
                result[row_index][col_index] = self.m[row_index][0] * rhs.m[0][col_index]
                    + self.m[row_index][1] * rhs.m[1][col_index]
                    + self.m[row_index][2] * rhs.m[2][col_index]
                    + self.m[row_index][3] * rhs.m[3][col_index];
            }
        }

        Mat4 { m: result }
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    fn mul(self, v: Vec4) -> Vec4 {
        Vec4 {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z + self.m[0][3] * v.w,

            y: self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z + self.m[1][3] * v.w,

            z: self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z + self.m[2][3] * v.w,

            w: self.m[3][0] * v.x + self.m[3][1] * v.y + self.m[3][2] * v.z + self.m[3][3] * v.w,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec4_approx_eq(a: Vec4, b: Vec4) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z) && approx_eq(a.w, b.w)
    }

    fn mat4_approx_eq(a: Mat4, b: Mat4) -> bool {
        (0..4).all(|i| (0..4).all(|j| approx_eq(a.m[i][j], b.m[i][j])))
    }

    #[test]
    fn test_identity() {
        let identity = Mat4::identity();
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        assert_eq!(identity * v, v);
    }

    #[test]
    fn test_translate() {
        let trans_mat = Mat4::translation(1.0, 2.0, 3.0);
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        let expected = Vec4::new(2.0, 4.0, 6.0, 1.0);
        assert_eq!(trans_mat * v, expected);
    }

    #[test]
    fn test_scale() {
        let scale_mat = Mat4::scale(1.0, 2.0, 3.0);
        let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
        // scale(sx, sy, sz) multiplies each component independently
        let expected = Vec4::new(1.0, 4.0, 9.0, 1.0);
        assert_eq!(scale_mat * v, expected);
    }

    #[test]
    fn test_rotate_x() {
        // Rotating (0, 1, 0) around X by 90° → (0, 0, 1)
        let rot_mat = Mat4::rotation_x(std::f32::consts::FRAC_PI_2);
        let result = rot_mat * Vec4::new(0.0, 1.0, 0.0, 1.0);
        let expected = Vec4::new(0.0, 0.0, 1.0, 1.0);
        assert!(vec4_approx_eq(result, expected), "got {result:?}");
    }

    #[test]
    fn test_rotate_y() {
        // Rotating (1, 0, 0) around Y by 180° → (-1, 0, 0)
        let rot_mat = Mat4::rotation_y(std::f32::consts::PI);
        let result = rot_mat * Vec4::new(1.0, 0.0, 0.0, 1.0);
        let expected = Vec4::new(-1.0, 0.0, 0.0, 1.0);
        assert!(vec4_approx_eq(result, expected), "got {result:?}");
    }

    #[test]
    fn test_rotate_z() {
        // Rotating (0, 1, 0) around Z by 180° → (0, -1, 0)
        let rot_mat = Mat4::rotation_z(std::f32::consts::PI);
        let result = rot_mat * Vec4::new(0.0, 1.0, 0.0, 1.0);
        let expected = Vec4::new(0.0, -1.0, 0.0, 1.0);
        assert!(vec4_approx_eq(result, expected), "got {result:?}");
    }

    #[test]
    fn test_transpose() {
        let m = Mat4 {
            m: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };
        let t = m.transpose();
        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(t.m[i][j], m.m[j][i]);
            }
        }
    }

    #[test]
    fn test_mat_mul_translations_add() {
        // Two translations composed should equal their sum
        let a = Mat4::translation(1.0, 2.0, 3.0);
        let b = Mat4::translation(4.0, 5.0, 6.0);
        let result = (a * b) * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let expected = Vec4::new(5.0, 7.0, 9.0, 1.0);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_inverse_of_translation() {
        let m = Mat4::translation(3.0, -1.0, 2.0);
        let inv = m.inverse().expect("translation matrix is invertible");
        let identity = m * inv;
        assert!(
            mat4_approx_eq(identity, Mat4::identity()),
            "got {identity:?}"
        );
    }

    #[test]
    fn test_inverse_of_scale() {
        let m = Mat4::scale(2.0, 4.0, 0.5);
        let inv = m.inverse().expect("scale matrix is invertible");
        let identity = m * inv;
        assert!(
            mat4_approx_eq(identity, Mat4::identity()),
            "got {identity:?}"
        );
    }

    #[test]
    fn test_inverse_singular_returns_none() {
        // Zero matrix has determinant 0 — not invertible
        let m = Mat4 { m: [[0.0; 4]; 4] };
        assert!(m.inverse().is_none());
    }

    // Perspective tests.
    // Visible points have camera-space z <= -near (camera looks down -z).
    // After perspective divide, NDC z should map [-near, -far] → [-1, +1].

    #[test]
    fn test_perspective_near_plane_maps_to_neg_one() {
        let (near, far) = (0.1, 100.0);
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, near, far);
        let clip = proj * Vec4::new(0.0, 0.0, -near, 1.0);
        assert!(
            approx_eq(clip.z / clip.w, -1.0),
            "near NDC z = {}",
            clip.z / clip.w
        );
    }

    #[test]
    fn test_perspective_far_plane_maps_to_pos_one() {
        let (near, far) = (0.1, 100.0);
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, near, far);
        let clip = proj * Vec4::new(0.0, 0.0, -far, 1.0);
        assert!(
            approx_eq(clip.z / clip.w, 1.0),
            "far NDC z = {}",
            clip.z / clip.w
        );
    }

    #[test]
    fn test_perspective_center_axis_stays_centered() {
        // A point on the camera axis (x=0, y=0) should project to NDC (0, 0)
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let clip = proj * Vec4::new(0.0, 0.0, -5.0, 1.0);
        assert!(approx_eq(clip.x / clip.w, 0.0));
        assert!(approx_eq(clip.y / clip.w, 0.0));
    }

    #[test]
    fn test_perspective_w_equals_neg_z() {
        // The perspective matrix sets w_clip = -z_cam, enabling perspective divide
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let clip = proj * Vec4::new(1.0, 2.0, -7.0, 1.0);
        assert!(approx_eq(clip.w, 7.0), "clip.w = {}", clip.w);
    }

    #[test]
    fn test_perspective_aspect_ratio_scales_x() {
        // With fov=90° and aspect=2, a point at (1, 0, -1) should have NDC x = 0.5
        // because f=1 and x_ndc = (f / aspect) * x / (-z) = 0.5 * 1 / 1 = 0.5
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 2.0, 0.1, 100.0);
        let clip = proj * Vec4::new(1.0, 0.0, -1.0, 1.0);
        assert!(
            approx_eq(clip.x / clip.w, 0.5),
            "ndc_x = {}",
            clip.x / clip.w
        );
    }
}
