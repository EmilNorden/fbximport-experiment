use crate::scene::mesh::Face;
use num::Zero;

/* Calculate surface normal for arbitrary polygon using Newell's method */
pub fn calculate_surface_normal(face: &Face, vertices: &Vec<glm::Vec3>) -> glm::Vec3 {
    let mut vertex_normal = glm::Vec3::zero();

    for i in 0..face.indices.len() {
        let current = vertices[face.indices[i] as usize];
        let next = vertices[face.indices[(i + 1) % face.indices.len()] as usize];

        vertex_normal.x += (current.y - next.y) * (current.z + next.z);
        vertex_normal.y += (current.z - next.z) * (current.x + next.x);
        vertex_normal.z += (current.x - next.x) * (current.y + next.y);
    }

    glm::normalize(vertex_normal)
}

/* Taken from https://stackoverflow.com/questions/2049582/how-to-determine-if-a-point-is-in-a-2d-triangle*/
pub fn is_point_in_triangle_2d(point: &glm::Vec2, v0: &glm::Vec2, v1: &glm::Vec2, v2: &glm::Vec2) -> bool {
    fn sign(v0: &glm::Vec2, v1: &glm::Vec2, v2: &glm::Vec2) -> f32 {
        (v0.x - v2.x) * (v1.y - v2.y) - (v1.x - v2.x) * (v0.y - v2.y)
    }

    let d1 = sign(point, v0, v1);
    let d2 = sign(point, v1, v2);
    let d3 = sign(point, v2, v0);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    !(has_neg && has_pos)
}

pub fn tri_contains_other_verts_2d<'a, I>(v0: &glm::Vec2, v1: &glm::Vec2, v2: &glm::Vec2, vertices: &'a mut I) -> bool
    where I: Iterator<Item = &'a glm::Vec2>
{
    for vertex in vertices {
        if vertex != v0 && vertex != v1 && vertex != v2 && is_point_in_triangle_2d(vertex, v0, v1, v2) {
            return true;
        }
    }
    false
}

pub fn project_triangle_into_2d(face: &Face, vertices: &Vec<glm::Vec3>) -> Vec<glm::Vec2> {
    let surface_normal = calculate_surface_normal(face, vertices);

    let absolute_normal = glm::abs(surface_normal);

    let mut project_axis_a = 0usize;
    let mut project_axis_b = 1usize;
    let mut inv = surface_normal.z;

    if absolute_normal.x > absolute_normal.y {
        if absolute_normal.x > absolute_normal.z {
            project_axis_a = 1;
            project_axis_b = 2;
            inv = surface_normal.x;
        }
    } else if absolute_normal.y > absolute_normal.z {
        project_axis_a = 2;
        project_axis_b = 0;
        inv = surface_normal.y;
    }

    if inv < 0.0 {
        std::mem::swap(&mut project_axis_a, &mut project_axis_b);
    }

    let mut plane_vertices = Vec::new();
    for i in 0..face.indices.len() {
        plane_vertices.push(glm::vec2(
            vertices[face.indices[i] as usize][project_axis_a],
            vertices[face.indices[i] as usize][project_axis_b],
        ));
    }
    plane_vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_contains_other_verts_2d_should_return_true_for_point_in_triangle() {
        // Arrange
        let v0 = glm::vec2(0.0, 0.0);
        let v1 = glm::vec2(0.0, -10.0);
        let v2 = glm::vec2(10.0, 10.0);

        let vertices = vec![glm::vec2(5.5, 5.5)];

        // Act
        let result = tri_contains_other_verts_2d(&v0, &v1, &v2, &mut vertices.iter());

        // Assert
        assert!(result);
    }

    #[test]
    fn tri_contains_other_verts_2d_should_return_true_for_point_directly_on_edge() {
        // Arrange
        let v0 = glm::vec2(0.0, 0.0);
        let v1 = glm::vec2(0.0, -10.0);
        let v2 = glm::vec2(10.0, -5.0);

        let vertices = vec![glm::vec2(0.0, -5.0)];

        // Act
        let result = tri_contains_other_verts_2d(&v0, &v1, &v2, &mut vertices.iter());

        // Assert
        assert!(result);
    }

    #[test]
    fn tri_contains_other_verts_2d_should_return_false_for_point_outside_triangle() {
        // Arrange
        let v0 = glm::vec2(0.0, 0.0);
        let v1 = glm::vec2(0.0, -10.0);
        let v2 = glm::vec2(10.0, -5.0);

        let vertices = vec![glm::vec2(-0.5, -5.0)];

        // Act
        let result = tri_contains_other_verts_2d(&v0, &v1, &v2, &mut vertices.iter());

        // Assert
        assert_eq!(result, false);
    }

    #[test]
    fn tri_contains_other_verts_2d_should_return_false_when_called_with_empty_vertex_iter() {
        // Arrange
        let v0 = glm::vec2(0.0, 0.0);
        let v1 = glm::vec2(0.0, -10.0);
        let v2 = glm::vec2(10.0, -5.0);

        let vertices = Vec::new();

        // Act
        let result = tri_contains_other_verts_2d(&v0, &v1, &v2, &mut vertices.iter());

        // Assert
        assert_eq!(result, false);
    }
}