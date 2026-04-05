use crate::maths::{mat4::Mat4, vec3::Vec3};

#[derive(Clone, Debug)]
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
            position: Vec3::new(0.0, 0.0, 5.0),
            rotation: Vec3::ZERO,
            fov: 0.5 * std::f32::consts::PI,
            aspect_ratio: width / height,
            near: 0.1,
            far: 100.0,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let f = self.forward();
        let r = self.right();
        let u = self.up();

        let p = self.position;

        Mat4 {
            m: [
                [r.x, r.y, r.z, -r.dot(p)],
                [u.x, u.y, u.z, -u.dot(p)],
                [-f.x, -f.y, -f.z, f.dot(p)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect_ratio, self.near, self.far)
    }

    pub fn forward(&self) -> Vec3 {
        let yaw = self.rotation.y;
        let pitch = self.rotation.x;

        Vec3::new(
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        )
        .normalise()
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0).cross(self.forward()).normalise()
    }

    pub fn up(&self) -> Vec3 {
        self.forward().cross(self.right()).normalise()
    }

    pub fn process_mouse(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.002;
        self.rotation.y -= dx * sensitivity; // yaw
        self.rotation.x += dy * sensitivity; // pitch

        // Clamp pitch to avoid flipping
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.rotation.x = self.rotation.x.clamp(-max_pitch, max_pitch);
    }
}
