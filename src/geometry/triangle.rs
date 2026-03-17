use crate::maths::mat4::Mat4;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use crate::maths::vec4::Vec4;

pub(crate) struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    pub normal: Vec3,
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

    pub fn project(
        &self,
        mat: Mat4,
        screen_width: f32,
        screen_height: f32,
    ) -> ((Vec2, f32), (Vec2, f32), (Vec2, f32)) {
        let project_vertex = |v: Vec3| {
            let clip = mat * Vec4::from_vec3(v, 1.0);
            let ndc_x = clip.x / clip.w;
            let ndc_y = clip.y / clip.w;
            let z = clip.z / clip.w;
            let screen_x = (ndc_x + 1.0) * 0.5 * screen_width;
            let screen_y = (1.0 - ndc_y) * 0.5 * screen_height;
            (Vec2::new(screen_x, screen_y), z)
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

    pub fn contains_point(&self, px: f32, py: f32) -> Option<(f32, f32, f32)> {
        let v0x = self.v2.x - self.v0.x;
        let v0y = self.v2.y - self.v0.y;

        let v1x = self.v1.x - self.v0.x;
        let v1y = self.v1.y - self.v0.y;

        let v2x = px - self.v0.x;
        let v2y = py - self.v0.y;

        let dot00 = v0x * v0x + v0y * v0y;
        let dot01 = v0x * v1x + v0y * v1y;
        let dot02 = v0x * v2x + v0y * v2y;
        let dot11 = v1x * v1x + v1y * v1y;
        let dot12 = v1x * v2x + v1y * v2y;

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);

        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        if u >= 0.0 && v >= 0.0 && u + v < 1.0 {
            Some((1.0 - u - v, v, u))
        } else {
            None
        }
    }
}
