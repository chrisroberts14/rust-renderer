use crate::maths::vec3::Vec3;

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<(usize, usize, usize)>,
    pub face_colors: Vec<[u8; 4]>,
    pub normals: Vec<Vec3>,
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vec3>,
        faces: Vec<(usize, usize, usize)>,
        face_colors: Vec<[u8; 4]>,
    ) -> Self {
        let normals = Self::compute_vertex_normals(&vertices, &faces);
        Self {
            vertices,
            faces,
            face_colors,
            normals,
        }
    }

    pub fn color_of(&self, face_index: usize) -> [u8; 4] {
        self.face_colors
            .get(face_index)
            .copied()
            .unwrap_or([255, 255, 255, 255])
    }

    fn compute_vertex_normals(vertices: &[Vec3], faces: &Vec<(usize, usize, usize)>) -> Vec<Vec3> {
        let mut normals = vec![Vec3::new(0.0, 0.0, 0.0); vertices.len()];

        for (i0, i1, i2) in faces {
            let v0 = vertices[*i0];
            let v1 = vertices[*i1];
            let v2 = vertices[*i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;

            let face_normal = edge1.cross(edge2).normalise();

            normals[*i0] = normals[*i0] + face_normal;
            normals[*i1] = normals[*i1] + face_normal;
            normals[*i2] = normals[*i2] + face_normal;
        }

        for n in &mut normals {
            *n = n.normalise();
        }

        normals
    }
}
