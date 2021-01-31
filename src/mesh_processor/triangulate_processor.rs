use crate::mesh_processor::MeshProcessor;
use crate::scene::mesh::{Mesh, Face};
use crate::polygon_utils::{calculate_surface_normal, is_point_in_triangle_2d, tri_contains_other_verts_2d};
use num::{Zero, Float};
use image::{RgbImage, Rgb};
use crate::scene::mesh::face_vertex_iterator::FaceVertexIterator;

mod face_triangulator;

pub struct TriangulateMeshProcessor {}

impl TriangulateMeshProcessor {
    fn triangle_area_2d(v1: &glm::Vec2, v2: &glm::Vec2, v3: &glm::Vec2) -> f32 {
        return (v1.x * (v3.y - v2.y)) + (v2.x * (v1.y - v3.y)) + (v3.x * (v2.y - v1.y));
    }

    fn is_point_on_left_side_of_line(line_v1: &glm::Vec2, line_v2: &glm::Vec2, point: &glm::Vec2) -> bool {
        TriangulateMeshProcessor::triangle_area_2d(line_v1, point, line_v2) > 0.0
    }

    pub fn new() -> Self {
        TriangulateMeshProcessor {}
    }

    // fn debug_face(face: &Face, vertices: &Vec<glm::Vec2>, name: &str) {
    fn debug_face(vertex_indices: Option<&[usize]>, vertices: &Vec<glm::Vec2>, name: &str) {
        let image_dimensions = glm::vec2(1024.0, 1024.0);

        let mut img = RgbImage::new(image_dimensions.x as u32, image_dimensions.y as u32);
        for y in 0..image_dimensions.y as u32 {
            for x in 0..image_dimensions.x as u32 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }

        fn range(count: usize) -> Vec<usize> {
            let mut indices = vec![0usize; count];
            for i in 0..indices.len() {
                indices[i] = i;
            }
            indices
        }

        // Self::debug_face_inner(face, vertices, &mut img);
        Self::debug_face_inner(vertex_indices.unwrap_or(range(vertices.len()).as_slice()), vertices, &mut img);

        img.save(format!("/Users/emil/temp/{}.png", name)).unwrap();
    }

    // fn debug_face_inner(face: &Face, vertices: &Vec<glm::Vec2>, img: &mut RgbImage) {
    fn debug_face_inner(vertex_indices: &[usize], vertices: &Vec<glm::Vec2>, img: &mut RgbImage) {
        // let mut vertices = Vec::with_capacity(face.indices.len());
        let mut smallest = glm::vec2(f32::max_value(), f32::max_value());
        let mut largest = glm::vec2(f32::min_value(), f32::min_value());
        // for index in &face.indices {

        // Uncomment to render face at mesh-scale
        /*for v in vertices {
            // let v = vertices[*index as usize];
            // vertices.push(v);

            smallest = glm::min(smallest, *v);
            largest = glm::max(largest, *v);
        }*/
        for i in vertex_indices {
            let v = vertices[*i];
            smallest = glm::min(smallest, v);
            largest = glm::max(largest, v);
        }

        smallest = smallest - glm::vec2(10.0, 10.0);
        largest = largest + glm::vec2(10.0, 10.0);


        let polygon_size = largest - smallest;

        let image_dimensions = glm::vec2(1024.0, 1024.0);

        fn get_slope(start: &glm::Vec2, end: &glm::Vec2) -> Option<f32> {
            if start.x == end.x {
                return None;
            }

            let slope = (end.y - start.y) / (end.x - start.x);
            if glm::abs(slope) > 100000.0 {
                // slope is steep enough to handle as vertical
                return None;
            }
            Some(slope)
        }

        fn get_intercept(start: &glm::Vec2, slope: Option<f32>) -> f32 {
            match slope {
                None => start.x,
                Some(x) => start.y - x * start.x
            }
        }

        // for i in 0..face.indices.len() {
        for i in 0..vertex_indices.len() {
            let from_index = vertex_indices[i];
            let to_index = vertex_indices[(i + 1) % vertex_indices.len()] as usize;
            let from_vertex = &vertices[from_index];
            let to_vertex = &vertices[to_index];

            let start = glm::vec2(
                ((from_vertex.x - smallest.x) / polygon_size.x) * (image_dimensions.x - 1.0),
                ((from_vertex.y - smallest.y) / polygon_size.y) * (image_dimensions.y - 1.0),
            );

            let end = glm::vec2(
                ((to_vertex.x - smallest.x) / polygon_size.x) * (image_dimensions.x - 1.0),
                ((to_vertex.y - smallest.y) / polygon_size.y) * (image_dimensions.y - 1.0),
            );

            let slope = get_slope(&start, &end);
            let intercept = get_intercept(&start, slope);

            let mut previous_distance = f32::max_value();

            let mut current_pos = glm::vec2(start.x, start.y);

            while glm::length(current_pos - end) < previous_distance {
                previous_distance = glm::length(current_pos - end);

                let base_increment = 0.1f32;
                match slope {
                    None => {
                        let mut step_increment = if start.y > end.y { -base_increment } else { base_increment };
                        let diff = end.y - start.y;
                        if glm::abs(diff) < glm::abs(step_increment) {
                            step_increment = diff;
                        }
                        current_pos.y += step_increment;
                    }
                    Some(value) => {
                        let mut step_increment = if start.x > end.x { -base_increment } else { base_increment };
                        let diff = end.x - start.x;
                        if glm::abs(diff) < glm::abs(step_increment) {
                            step_increment = diff;
                        }

                        current_pos.x += step_increment;
                        current_pos.y = value * current_pos.x + intercept;
                    }
                };

                img.put_pixel(current_pos.x as u32, current_pos.y as u32, Rgb([0, 0, 0]));
            }
        }
    }

    fn project_triangle_into_2d(face: &Face, vertices: &Vec<glm::Vec3>) -> Vec<glm::Vec2> {
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

    /*fn tri_contains_other_verts_2d(v0: &glm::Vec2, v1: &glm::Vec2, v2: &glm::Vec2, face: &Face, vertices: &Vec<glm::Vec2>) -> bool {
        for i in 0..face.indices.len() {
            let vertex = &vertices[i];

            if vertex != v0 && vertex != v1 && vertex != v2 && is_point_in_triangle_2d(vertex, v0, v1, v2) {
                return true;
            }
        }

        false
    }*/
}

impl MeshProcessor for TriangulateMeshProcessor {
    fn process(&self, mesh: &mut Mesh) {
        let mut new_faces = Vec::new();
        let mut img = RgbImage::new(1024, 1024);
        for y in 0..1024 {
            for x in 0..1024 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }

        let mut face_counter = 0;
        for face in &mesh.faces {
            face_counter += 1;
            if face.indices.len() == 3 {
                println!("Skipping face {}. Already a triangle", face_counter);
                continue;
            }
            println!("Triangulating face {} of {}", face_counter, mesh.faces.len());
            let plane_vertices = Self::project_triangle_into_2d(face, &mesh.vertices);

            // Self::debug_face(face, &plane_vertices, &*format!("{}_face{}_full", mesh.name, face_counter));
            Self::debug_face(None, &plane_vertices, &*format!("{}_face{}_full", mesh.name, face_counter));

            let mut clipped_vertices = vec![false; face.indices.len()];

            let mut polygon_size = face.indices.len();
            while polygon_size > 3 {
                // FIND EAR
                println!("Polygons remaining: {}", polygon_size);
                for i in 0..face.indices.len() {
                    if clipped_vertices[i] {
                        continue;
                    }

                    let mut previous = if i == 0 { face.indices.len() - 1 } else { i - 1 };
                    while clipped_vertices[previous] {
                        previous = if previous == 0 { face.indices.len() - 1 } else { previous - 1 };
                    }

                    let mut next = (i + 1) % face.indices.len();
                    while clipped_vertices[next] {
                        next = (next + 1) % face.indices.len();
                    }

                    let v0 = plane_vertices[previous];
                    let v1 = plane_vertices[i];
                    let v2 = plane_vertices[next];

                    if Self::is_point_on_left_side_of_line(&v0, &v2, &v1) {
                        // Assuming CCW  winding, the point should be on the right side.
                        // Move on to the next vertex in the polygon
                        continue;
                    }

                    if tri_contains_other_verts_2d(&v0, &v1, &v2,
                                                   &mut FaceVertexIterator::from(
                                                       &mut face.indices.iter(),
                                                       &plane_vertices)) {
                        continue;
                    }

                    new_faces.push(Face::new(vec![face.indices[previous], face.indices[i], face.indices[next]]));

                    // Self::debug_face(&new_faces[new_faces.len() - 1], &plane_vertices, &*format!("{}_face{}_triangle{}", mesh.name, face_counter, new_faces.len()));
                    Self::debug_face(Some([previous, i, next].as_slice()), &plane_vertices, &*format!("{}_face{}_triangle{}", mesh.name, face_counter, new_faces.len()));
                    // Self::debug_face_inner(&new_faces[new_faces.len() - 1], &plane_vertices, &mut img);
                    Self::debug_face_inner([previous, i, next].as_slice(), &plane_vertices, &mut img);
                    clipped_vertices[i] = true;
                    polygon_size -= 1;
                    /*while clipped_vertices[previous] && i != previous {

                    }*/
                    if polygon_size < 3 {
                        break;
                    }
                }
            }
        }

        img.save(format!("/Users/emil/temp/{}_result.png", mesh.name)).unwrap();
        mesh.faces = new_faces;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glm::sin;
    use std::f32::consts::PI;
    use crate::fbx::import_fbx;

    #[test]
    fn process_should_handle_convex_quad() {
        // Arrange
        /*let vertices = vec![
            glm::vec3(0.0, 0.0, 0.0),
            glm::vec3(0.0, -10.0, 0.0),
            glm::vec3(10.0, -10.0, 0.0),
            glm::vec3(10.0, 0.0, 0.0),
        ];

        let faces = vec![
            Face::new(vec![0, 1, 2, 3])
        ];*/

        // STAR
        let mut vertices = Vec::new();
        let radians_step = 1.25663706 / 2.0;
        let mut current_angle = 0.0f32;
        for i in 0..10 {
            let radius = if i % 2 == 0 { 6.0f32 } else { 2.0f32 };
            let x = glm::sin(current_angle) * radius;
            let y = glm::cos(current_angle) * radius;

            vertices.push(glm::vec3(x, y, 0.0));

            current_angle -= radians_step;
        }

        let faces = vec![
            Face::new(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
        ];

        let mut mesh = Mesh::new("star".to_string(), vertices, faces);

        let mut sut = TriangulateMeshProcessor::new();

        // Act
        sut.process(&mut mesh);

        // Assert
        // assert_eq!(mesh.faces.len(), 2);

        let face1 = &mesh.faces[0];
        /*assert_eq!(face1.indices[0], 3);
        assert_eq!(face1.indices[1], 0);
        assert_eq!(face1.indices[2], 1);*/

        let face2 = &mesh.faces[1];
        /*assert_eq!(face2.indices[0], 3);
        assert_eq!(face2.indices[1], 1);
        assert_eq!(face2.indices[2], 2);*/
    }

    #[test]
    fn process_should_handle_concave_quad() {
        // Arrange
        let vertices = vec![
            glm::vec3(9.5, -9.5, 0.0),
            glm::vec3(0.0, -10.0, 0.0),
            glm::vec3(10.0, -10.0, 0.0),
            glm::vec3(10.0, 0.0, 0.0),
        ];

        let faces = vec![
            Face::new(vec![0, 1, 2, 3])
        ];

        let mut mesh = Mesh::new("poly1".to_string(), vertices, faces);

        let sut = TriangulateMeshProcessor::new();

        // Act
        sut.process(&mut mesh);

        // Assert
        // assert_eq!(mesh.faces.len(), 2);

        let face1 = &mesh.faces[0];
        /*assert_eq!(face1.indices[0], 0);
        assert_eq!(face1.indices[1], 1);
        assert_eq!(face1.indices[1], 2);*/

        let face2 = &mesh.faces[1];
        /*assert_eq!(face2.indices[0], 0);
        assert_eq!(face2.indices[1], 2);
        assert_eq!(face2.indices[1], 3);*/
    }
}