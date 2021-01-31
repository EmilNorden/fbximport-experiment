use crate::fbx::node::NodeRecord;
use crate::scene::Scene;
use crate::fbx::node_collection::{NodeCollection, Error};
use crate::fbx::property::PropertyRecordType;
use crate::scene::mesh::{Mesh, Face};
use num::abs;
use std::fs::File;
use std::path::Path;
use std::io::{Write, Cursor};
use std::slice::Iter;

struct FaceIterator<'a, I>
where
    I: Iterator<Item = &'a i32>
{
    indices: &'a mut I,
}

impl<'a, I: Iterator<Item = &'a i32>> FaceIterator<'a, I> {
    pub fn from(indices: &'a mut I) -> Self {
        FaceIterator {
            indices
        }
    }
}

impl<'a, I: Iterator<Item = &'a i32>> Iterator for FaceIterator<'a, I> {
    type Item = Face;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut indices = Vec::<i32>::new();

        while let Some(index) = self.indices.next() {
            if *index < 0 {
                indices.push(*index ^ -1);
                break;
            }

            indices.push(*index);
        }

        if indices.is_empty() {
            return None;
        }

        Some(Face::new(indices))
    }
}

struct Tuples3<I> {
    original: I,
}

impl<I> Iterator for Tuples3<I> where I: Iterator {
    type Item = (I::Item, I::Item, I::Item);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(t1) = self.original.next() {
            if let Some(t2) = self.original.next() {
                if let Some(t3) = self.original.next() {
                    return Some((t1, t2, t3));
                }
            }
        }

        return None;
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.original.size_hint() {
            (lower, Some(upper)) => (lower, Some(upper / 3)),
            h @ (_, _) => h
        }
    }
}

fn tuples3<I: Iterator>(iterator: I) -> Tuples3<I> {
    Tuples3 { original: iterator }
}

fn get_faces(geometry: &NodeRecord) -> Vec<Face> {
    let indices_node = match geometry.children.get("PolygonVertexIndex") {
        Ok(v) => v,
        Err(e) => panic!("sssss")
    };

    let mut indices = match &indices_node.properties[0] {
        PropertyRecordType::SignedInt32Array(v) => v.clone(),
        _ => panic!("Unexpected data in indices node")
    };

    let mut faces = Vec::new();
    for face in FaceIterator::from(&mut indices.iter()) {
        faces.push(face);
    }

    faces
}

pub(super) fn import(nodes: NodeCollection) -> Option<Scene> {
    let objects_node = match nodes.get("Objects") {
        Ok(node) => node,
        Err(_) => panic!("woop")
    };

    let geometry = objects_node.children.get_multiple("Geometry");

    if geometry.is_none() {
        // No meshes to import
        return None;
    }

    let mut meshes = Vec::new();
    for geom in geometry.unwrap() {
        // 3rd property should be "Mesh"
        if geom.properties.len() < 3 {
            continue;
        }

        let name = match &geom.properties[1] {
            PropertyRecordType::String(str) => Some(str),
            _ => None
        }.unwrap().clone();

        let object_type = match &geom.properties[2] {
            PropertyRecordType::String(str) => Some(str),
            _ => None
        };

        if object_type.is_none() || object_type.unwrap() != "Mesh" {
            continue;
        }

        let vertices_node = match geom.children.get("Vertices") {
            Ok(v) => v,
            Err(e) => panic!("Errorrrrr!")
        };

        let coordinates = match &vertices_node.properties[0] {
            PropertyRecordType::DoubleArray(arr) => arr,
            _ => panic!("Unexpected data in vertex node")
        };

        let vertices: Vec<glm::Vec3> = tuples3(coordinates.iter()
            .map(|x| *x as f32)).map(|x| glm::vec3(x.0, x.1, x.2)).collect();


        meshes.push(
            Mesh::new(
                name,
                vertices,
                get_faces(geom)
            ));
    }

    Some(Scene::new(meshes))
}