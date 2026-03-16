use crate::maths::vec3::Vec3;

pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<(usize, usize, usize)>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, faces: Vec<(usize, usize, usize)>) -> Self {
        Self { vertices, faces }
    }
}
