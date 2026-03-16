use crate::maths::{mat4::Mat4, vec3::Vec3};

pub struct Camera {
    pub position: Vec3,
    pub rotation: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 1.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            fov: 0.5 * std::f32::consts::PI,
            aspect_ratio: width / height,
            near: 0.1,
            far: 100.0,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let rx = -self.rotation.x;
        let ry = -self.rotation.y;
        let rz = -self.rotation.z;

        let px = -self.position.x;
        let py = -self.position.y;
        let pz = -self.position.z;

        let rot_x = Mat4::rotation_x(rx);
        let rot_y = Mat4::rotation_y(ry);
        let rot_z = Mat4::rotation_z(rz);

        let trans = Mat4::translation(px, py, pz);

        rot_x * rot_y * rot_z * trans
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect_ratio, self.near, self.far)
    }

    /// Move the camera by a given amount
    /// This will add the vec given here to the position vector of the camera
    pub fn move_camera(&mut self, vec: Vec3) {
        self.position = self.position + vec;
    }
}
