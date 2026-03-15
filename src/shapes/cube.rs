use crate::{
    framebuffer::Framebuffer,
    maths::vec3::Vec3,
    shapes::{Shape, line::Line},
};

const CUBE_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

pub struct Cube {
    pub vertices: [Vec3; 8],
}

impl Cube {
    pub fn new(size: f32) -> Self {
        let s = size / 2.0;
        Self {
            vertices: [
                Vec3 {
                    x: -s,
                    y: -s,
                    z: -s,
                },
                Vec3 { x: s, y: -s, z: -s },
                Vec3 { x: s, y: s, z: -s },
                Vec3 { x: -s, y: s, z: -s },
                Vec3 { x: -s, y: -s, z: s },
                Vec3 { x: s, y: -s, z: s },
                Vec3 { x: s, y: s, z: s },
                Vec3 { x: -s, y: s, z: s },
            ],
        }
    }

    pub fn rotated(&self, rot_x: f32, rot_y: f32, rot_z: f32) -> [Vec3; 8] {
        let mut new_vertices = [Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }; 8];
        for (i, v) in self.vertices.iter().enumerate() {
            let rotated = v.rotate_x(rot_x).rotate_y(rot_y).rotate_z(rot_z);
            new_vertices[i] = rotated;
        }
        new_vertices
    }
}

impl Shape for Cube {
    fn draw(&self, framebuffer: &mut Framebuffer) {
        // Example rotation angles (radians)
        let rot_x = 0.5; // rotate 0.5 rad around X
        let rot_y = 0.3; // rotate 0.3 rad around Y
        let rot_z = 0.0; // no rotation around Z

        let rotated_vertices = self.rotated(rot_x, rot_y, rot_z);

        for (start, end) in CUBE_EDGES.iter() {
            let p0 = rotated_vertices[*start].project_to_2d(framebuffer.width, framebuffer.height);
            let p1 = rotated_vertices[*end].project_to_2d(framebuffer.width, framebuffer.height);
            let line = Line::new(p0, p1);
            line.draw(framebuffer);
        }
    }
}
