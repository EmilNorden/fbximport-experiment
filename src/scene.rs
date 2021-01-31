use crate::scene::mesh::Mesh;

pub mod mesh;

pub struct Scene {
    pub(crate) meshes: Vec<Mesh>,
}

impl Scene {
    pub fn new(meshes: Vec<Mesh>) -> Self {
        Scene {
            meshes
        }
    }
}

