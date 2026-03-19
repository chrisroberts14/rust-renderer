use crate::geometry::mesh::Mesh;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;
use std::f32::consts::PI;

#[allow(dead_code)]
pub struct Sphere;

#[allow(dead_code)]
impl Sphere {
    pub fn mesh(radius: f32, stacks: u32, slices: u32) -> Mesh {
        let mut vertices = vec![];
        let mut uvs = vec![];
        let mut faces = vec![];
        let mut uv_faces = vec![];
        let mut face_colors = vec![];

        let color = [180, 180, 220, 255];

        // Generate vertices and UVs together (same indexing)
        for stack in 0..=stacks {
            let phi = PI * stack as f32 / stacks as f32; // 0 to PI
            for slice in 0..=slices {
                let theta = 2.0 * PI * slice as f32 / slices as f32; // 0 to 2PI
                vertices.push(Vec3 {
                    x: radius * phi.sin() * theta.cos(),
                    y: radius * phi.cos(),
                    z: radius * phi.sin() * theta.sin(),
                });
                uvs.push(Vec2::new(
                    slice as f32 / slices as f32,
                    stack as f32 / stacks as f32,
                ));
            }
        }

        // Generate faces — uv_faces mirror faces since indexing is shared
        for stack in 0..stacks {
            for slice in 0..slices {
                let top_left = stack * (slices + 1) + slice;
                let top_right = top_left + 1;
                let bottom_left = top_left + (slices + 1);
                let bottom_right = bottom_left + 1;

                let tl = top_left as usize;
                let tr = top_right as usize;
                let bl = bottom_left as usize;
                let br = bottom_right as usize;

                // Each quad becomes 2 triangles (counter-clockwise winding)
                faces.push((tl, tr, bl));
                uv_faces.push((tl, tr, bl));
                face_colors.push(color);
                faces.push((tr, br, bl));
                uv_faces.push((tr, br, bl));
                face_colors.push(color);
            }
        }

        Mesh::new(vertices, faces, face_colors, uvs, uv_faces)
    }
}
