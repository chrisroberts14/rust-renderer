use crate::maths::vec3::Vec3;

pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<(usize, usize, usize)>,
    pub face_colors: Vec<[u8; 4]>,
}

impl Mesh {
    pub fn new(
        vertices: Vec<Vec3>,
        faces: Vec<(usize, usize, usize)>,
        face_colors: Vec<[u8; 4]>,
    ) -> Self {
        Self {
            vertices,
            faces,
            face_colors,
        }
    }

    pub fn color_of(&self, face_index: usize) -> [u8; 4] {
        self.face_colors
            .get(face_index)
            .copied()
            .unwrap_or([255, 255, 255, 255])
    }
}
