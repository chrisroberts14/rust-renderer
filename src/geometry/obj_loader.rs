use std::fs;
use std::io;
use std::path::Path;

use crate::geometry::mesh::Mesh;
use crate::maths::vec3::Vec3;

pub struct ObjLoader;

impl ObjLoader {
    /// Load a .obj file into a Mesh.  All faces are given `color`.
    /// Supports `v`, `f` lines; face indices may be `v`, `v/vt`, `v//vn`, or `v/vt/vn`.
    /// Polygons with more than 3 vertices are fan-triangulated.
    pub fn load(path: &Path, color: [u8; 4]) -> io::Result<Mesh> {
        let source = fs::read_to_string(path)?;

        let mut vertices: Vec<Vec3> = Vec::new();
        let mut faces: Vec<(usize, usize, usize)> = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if let Some(stripped_line) = line.strip_prefix("v ") {
                let coords: Vec<f32> = stripped_line
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if coords.len() >= 3 {
                    vertices.push(Vec3::new(coords[0], coords[1], coords[2]));
                }
            } else if let Some(stripped_line) = line.strip_prefix("f ") {
                // Each token may be "v", "v/vt", "v//vn", or "v/vt/vn"
                let indices: Vec<usize> = stripped_line
                    .split_whitespace()
                    .filter_map(|token| {
                        // Take just the first number before any '/'
                        let v_str = token.split('/').next()?;
                        let idx: i32 = v_str.parse().ok()?;
                        // OBJ indices are 1-based; negative indices are relative
                        if idx > 0 {
                            Some((idx - 1) as usize)
                        } else {
                            Some((vertices.len() as i32 + idx) as usize)
                        }
                    })
                    .collect();

                // Fan triangulation: (0,1,2), (0,2,3), (0,3,4), ...
                for i in 1..indices.len().saturating_sub(1) {
                    faces.push((indices[0], indices[i], indices[i + 1]));
                }
            }
        }

        let face_colors = vec![color; faces.len()];
        Ok(Mesh::new(vertices, faces, face_colors))
    }
}
