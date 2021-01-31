pub struct FaceVertexIterator<'a, IndexIterator, VecType>
    where
        IndexIterator: Iterator<Item = &'a i32>
{
    indices: &'a mut IndexIterator,
    vertices: &'a Vec<VecType>
}

impl<'a, IndexIterator: Iterator<Item = &'a i32>, VecType> FaceVertexIterator<'a, IndexIterator, VecType> {
    pub fn from(indices: &'a mut IndexIterator, vertices: &'a Vec<VecType>) -> Self {
        FaceVertexIterator {
            indices,
            vertices
        }
    }
}

impl<'a, IndexIterator: Iterator<Item = &'a i32>, VecType> Iterator for FaceVertexIterator<'a, IndexIterator, VecType>
{
    type Item = &'a VecType;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.indices.next() {
            return Some(&self.vertices[*index as usize]);
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_vertex_iterator_should_handle_empty_input() {
        let indices = Vec::<i32>::new();
        let vertices = Vec::<glm::Vec2>::new();
    }
}