use crate::polygon_utils::project_triangle_into_2d;
use crate::scene::mesh::Face;

struct FaceTriangulator<'a> {
    face: &'a Face,
    plane_vertices: Vec<glm::Vec2>,
    clipped_vertices: Vec<bool>,
    remaining_polygons: usize,
}

impl<'a> FaceTriangulator<'a> {
    pub fn new(face: &'a Face, vertices: &'a Vec<glm::Vec3>) -> Self {
        let plane_vertices = project_triangle_into_2d(face, vertices);
        FaceTriangulator {
            face,
            plane_vertices,
            clipped_vertices: vec![false; face.indices.len()],
            remaining_polygons: face.indices.len(),
        }
    }
}

impl Iterator for FaceTriangulator<'_> {
    type Item = Face;

    fn next(&mut self) -> Option<Self::Item> {
        if self.face.indices.len() == 3 {
            return Some(self.face.clone());
        }

        // if self.remaining_polygons

        None
    }
}