use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<(usize, usize, usize)>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub uv_faces: Vec<(usize, usize, usize)>,
    pub aabb_bounding_box: Option<(Vec3, Vec3)>,
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vec3>,
        faces: Vec<(usize, usize, usize)>,
        uvs: Vec<Vec2>,
        uv_faces: Vec<(usize, usize, usize)>,
    ) -> Self {
        let normals = Self::compute_vertex_normals(&vertices, &faces);
        let aabb_bounding_box = Self::compute_aabb(&vertices);
        Self {
            vertices,
            faces,
            normals,
            uvs,
            uv_faces,
            aabb_bounding_box
        }
    }

    fn compute_aabb(vertices: &[Vec3]) -> Option<(Vec3, Vec3)> {
        if vertices.is_empty() {
            return None;
        }

        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

        for &v in vertices {
            min = min.min(v);
            max = max.max(v);
        }

        Some((min, max))
    }

    fn compute_vertex_normals(vertices: &[Vec3], faces: &[(usize, usize, usize)]) -> Vec<Vec3> {
        let mut normals = vec![Vec3::ZERO; vertices.len()];

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
