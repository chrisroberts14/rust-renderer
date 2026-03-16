use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::maths::vec4::Vec4;

pub(crate) struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    normal: Vec3,
}

impl Triangle {
    pub(crate) fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Triangle {
        let v0v1 = Vec3 {
            x: v1.x - v0.x,
            y: v1.y - v0.y,
            z: v1.z - v0.z,
        };
        let v0v2 = Vec3 {
            x: v2.x - v0.x,
            y: v2.y - v0.y,
            z: v2.z - v0.z,
        };
        let normal = Vec3 {
            x: v0v1.y * v0v2.z - v0v1.z * v0v2.y,
            y: v0v1.z * v0v2.x - v0v1.x * v0v2.z,
            z: v0v1.x * v0v2.y - v0v1.y * v0v2.x,
        }
        .normalise();
        Triangle { v0, v1, v2, normal }
    }

    pub fn transform(&self, mat: Mat4) -> Triangle {
        let new_v0 = (mat * Vec4::from_vec3(self.v0, 1.0)).to_vec3();
        let new_v1 = (mat * Vec4::from_vec3(self.v1, 1.0)).to_vec3();
        let new_v2 = (mat * Vec4::from_vec3(self.v2, 1.0)).to_vec3();
        Triangle::new(new_v0, new_v1, new_v2)
    }

    pub fn project(&self, mat: Mat4, screen_width: f32, screen_height: f32) -> (Vec2, Vec2, Vec2) {
        let project_vertex = |v: Vec3| {
            let clip = mat * Vec4::from_vec3(v, 1.0);
            let ndc_x = clip.x / clip.w;
            let ndc_y = clip.y / clip.w;
            let _z = clip.z / clip.w; // keep for depth buffer later
            let screen_x = (ndc_x + 1.0) * 0.5 * screen_width;
            let screen_y = (1.0 - ndc_y) * 0.5 * screen_height; // y flipped
            Vec2::new(screen_x, screen_y)
        };

        (
            project_vertex(self.v0),
            project_vertex(self.v1),
            project_vertex(self.v2),
        )
    }

    pub fn is_backface(&self, camera_pos: Vec3) -> bool {
        let to_camera = Vec3 {
            x: camera_pos.x - self.v0.x,
            y: camera_pos.y - self.v0.y,
            z: camera_pos.z - self.v0.z,
        };
        self.normal.dot(to_camera) < 0.0
    }

    pub fn bounding_box(&self) -> (Vec2, Vec2) {
        let min_x = self.v0.x.min(self.v1.x).min(self.v2.x);
        let max_x = self.v0.x.max(self.v1.x).max(self.v2.x);
        let min_y = self.v0.y.min(self.v1.y).min(self.v2.y);
        let max_y = self.v0.y.max(self.v1.y).max(self.v2.y);
        (Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
    }

    pub fn contains_point(&self, p: Vec2) -> bool {
        let v0v1 = Vec3 {
            x: self.v1.x - self.v0.x,
            y: self.v1.y - self.v0.y,
            z: self.v1.z - self.v0.z,
        };
        let v0v2 = Vec3 {
            x: self.v2.x - self.v0.x,
            y: self.v2.y - self.v0.y,
            z: self.v2.z - self.v0.z,
        };
        let v0p = Vec3 {
            x: p.x - self.v0.x,
            y: p.y - self.v0.y,
            z: 0.0, // assume point is in the same plane
        };

        let dot00 = v0v2.dot(v0v2);
        let dot01 = v0v2.dot(v0v1);
        let dot02 = v0v2.dot(v0p);
        let dot11 = v0v1.dot(v0v1);
        let dot12 = v0v1.dot(v0p);

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        u >= 0.0 && v >= 0.0 && (u + v) < 1.0
    }
}
