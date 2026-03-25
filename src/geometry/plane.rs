use crate::geometry::mesh::Mesh;
use crate::maths::vec3::Vec3;

pub struct Plane;

impl Plane {
    /// Creates a flat mesh in the XZ plane, centred at the origin, facing up (+Y).
    /// `subdivisions` controls how many rows and columns of quads the plane is split into.
    /// Higher values prevent precision issues when the plane is large or viewed from close range.
    pub fn mesh(size: f32, subdivisions: u32) -> Mesh {
        let n = subdivisions as usize;
        let step = size / n as f32;
        let half = size / 2.0;

        let mut vertices = Vec::new();
        for row in 0..=n {
            for col in 0..=n {
                let x = col as f32 * step - half;
                let z = row as f32 * step - half;
                vertices.push(Vec3::new(x, 0.0, z));
            }
        }

        let mut faces = Vec::new();
        let stride = n + 1;
        for row in 0..n {
            for col in 0..n {
                let i0 = row * stride + col;
                let i1 = i0 + 1;
                let i2 = i0 + stride;
                let i3 = i2 + 1;
                // Both triangles wind to produce a normal pointing up (+Y).
                faces.push((i0, i3, i1));
                faces.push((i0, i2, i3));
            }
        }

        Mesh::new(vertices, faces, vec![], vec![])
    }
}
