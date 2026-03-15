use crate::maths::{mat4::Mat4, vec3::Vec3};

pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: Vec3::new(1.0, 1.0, 1.0),
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
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}
