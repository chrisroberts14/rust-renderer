use crate::maths::vec3::Vec3;

pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub edges: Vec<(usize, usize)>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, edges: Vec<(usize, usize)>) -> Self {
        Self { vertices, edges }
    }
}
