use crate::geometry::mesh::Mesh;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use std::f32::consts::PI;

pub struct Sphere;

impl Sphere {
    pub fn mesh(radius: f32, stacks: u32, slices: u32) -> Mesh {
        let mut vertices = vec![];
        let mut uvs = vec![];
        let mut faces = vec![];

        for stack in 0..=stacks {
            let phi = PI * stack as f32 / stacks as f32; // 0 to PI
            let sin_phi = phi.sin();
            let y = radius * phi.cos();
            for slice in 0..=slices {
                let theta = 2.0 * PI * slice as f32 / slices as f32; // 0 to 2PI
                vertices.push(Vec3 {
                    x: radius * sin_phi * theta.cos(),
                    y,
                    z: radius * sin_phi * theta.sin(),
                });
                uvs.push(Vec2::new(
                    slice as f32 / slices as f32,
                    stack as f32 / stacks as f32,
                ));
            }
        }

        for stack in 0..stacks {
            for slice in 0..slices {
                let top_left = (stack * (slices + 1) + slice) as usize;
                let top_right = top_left + 1;
                let bottom_left = top_left + (slices + 1) as usize;
                let bottom_right = bottom_left + 1;

                // Each quad becomes 2 triangles (counter-clockwise winding)
                faces.push((top_left, top_right, bottom_left));
                faces.push((top_right, bottom_right, bottom_left));
            }
        }

        // UV and vertex indices are shared, so uv_faces == faces
        let uv_faces = faces.clone();
        Mesh::new(vertices, faces, uvs, uv_faces)
    }
}
