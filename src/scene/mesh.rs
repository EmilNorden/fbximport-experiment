
pub mod face_vertex_iterator;

#[derive(Clone)]
pub struct Face {
    pub(crate) indices: Vec<i32>
}

impl Face {
    pub fn new(indices: Vec<i32>) -> Self {
        Face{
            indices
        }
    }
}

pub struct Mesh {
    pub(crate) vertices: Vec<glm::Vec3>,
    pub(crate) faces: Vec<Face>,
    pub(crate) name: String,
    // pub(crate) indices: Vec<i32>,
}

impl Mesh {
    pub fn new(name: String, vertices: Vec<glm::Vec3>, faces: Vec<Face>) -> Self {
        Mesh {
            vertices,
            faces,
            name,
        }
    }
}