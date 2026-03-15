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

        let edges = vec![
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

        Mesh::new(vertices, edges)
    }
}
