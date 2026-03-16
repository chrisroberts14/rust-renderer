use crate::geometry::mesh::Mesh;
use crate::maths::vec3::Vec3;

pub struct Cube;

impl Cube {
    pub fn mesh(size: f32) -> Mesh {
        let s = size / 2.0;

        let vertices = vec![
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
        ];

        let faces = vec![
            // Front (z = -s)
            (0, 2, 1),
            (0, 3, 2),
            // Back (z = +s)
            (4, 5, 6),
            (4, 6, 7),
            // Left (x = -s)
            (4, 3, 0),
            (4, 7, 3),
            // Right (x = +s)
            (1, 2, 6),
            (1, 6, 5),
            // Bottom (y = -s)
            (0, 1, 5),
            (0, 5, 4),
            // Top (y = +s)
            (3, 6, 2),
            (3, 7, 6),
        ];

        let red = [220, 60, 60, 255];
        let green = [60, 180, 60, 255];
        let blue = [60, 60, 220, 255];
        let yellow = [220, 200, 50, 255];
        let cyan = [50, 200, 200, 255];
        let white = [220, 220, 220, 255];

        let face_colors = vec![
            red, red, green, green, blue, blue, yellow, yellow, cyan, cyan, white, white,
        ];

        Mesh::new(vertices, faces, face_colors)
    }
}
