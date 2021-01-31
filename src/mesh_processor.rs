use crate::scene::mesh::Mesh;

pub mod triangulate_processor;

pub trait MeshProcessor {
    fn process(&self, mesh: &mut Mesh);
}