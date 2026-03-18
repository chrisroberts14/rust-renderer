use std::fs;
use std::io;
use std::path::Path;

use crate::geometry::mesh::Mesh;
use crate::maths::vec2::Vec2;
use crate::maths::vec3::Vec3;

pub struct ObjLoader;

impl ObjLoader {
    /// Load a .obj file into a Mesh.  All faces are given `color`.
    /// Supports `v`, `vt`, `f` lines; face indices may be `v`, `v/vt`, `v//vn`, or `v/vt/vn`.
    /// Polygons with more than 3 vertices are fan-triangulated.
    pub fn load(path: &Path, color: [u8; 4]) -> io::Result<Mesh> {
        let source = fs::read_to_string(path)?;

        let mut vertices: Vec<Vec3> = Vec::new();
        let mut uvs: Vec<Vec2> = Vec::new();
        let mut faces: Vec<(usize, usize, usize)> = Vec::new();
        let mut uv_faces: Vec<(usize, usize, usize)> = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if let Some(stripped_line) = line.strip_prefix("vt ") {
                let coords: Vec<f32> = stripped_line
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 2 {
                    uvs.push(Vec2::new(coords[0], coords[1]));
                }
            } else if let Some(stripped_line) = line.strip_prefix("v ") {
                let coords: Vec<f32> = stripped_line
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 3 {
                    vertices.push(Vec3::new(coords[0], coords[1], coords[2]));
                }
            } else if let Some(stripped_line) = line.strip_prefix("f ") {
                // Each token may be "v", "v/vt", "v//vn", or "v/vt/vn"
                // Parse into (vertex_idx, uv_idx) pairs
                let tokens: Vec<(usize, Option<usize>)> = stripped_line
                    .split_whitespace()
                    .filter_map(|token| {
                        let parts: Vec<&str> = token.split('/').collect();

                        let idx: i32 = parts[0].parse().ok()?;
                        let v_idx = if idx > 0 {
                            (idx - 1) as usize
                        } else {
                            (vertices.len() as i32 + idx) as usize
                        };

                        let uv_idx = parts
                            .get(1)
                            .and_then(|s| {
                                if s.is_empty() {
                                    None
                                } else {
                                    s.parse::<i32>().ok()
                                }
                            })
                            .map(|idx| {
                                if idx > 0 {
                                    (idx - 1) as usize
                                } else {
                                    (uvs.len() as i32 + idx) as usize
                                }
                            });

                        Some((v_idx, uv_idx))
                    })
                    .collect();

                // Fan triangulation: (0,1,2), (0,2,3), (0,3,4), ...
                for i in 1..tokens.len().saturating_sub(1) {
                    let (v0, uv0) = tokens[0];
                    let (v1, uv1) = tokens[i];
                    let (v2, uv2) = tokens[i + 1];
                    faces.push((v0, v1, v2));
                    uv_faces.push((uv0.unwrap_or(0), uv1.unwrap_or(0), uv2.unwrap_or(0)));
                }
            }
        }

        let face_colors = vec![color; faces.len()];
        Ok(Mesh::new(vertices, faces, face_colors, uvs, uv_faces))
    }
}
