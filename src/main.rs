#![feature(seek_convenience)]
#![feature(bufreader_seek_relative)]
#![feature(array_methods)]

use crate::fbx::import_fbx;
use crate::mesh_processor::triangulate_processor::TriangulateMeshProcessor;
use crate::mesh_processor::MeshProcessor;

mod fbx;
mod scene;
mod mesh_processor;
mod polygon_utils;

fn main() {
    let path = "/Users/emil/Downloads/pig.fbx";

    let mut processors = Vec::<Box<dyn MeshProcessor>>::new();
    processors.push(Box::new(TriangulateMeshProcessor{}));

    let _model = import_fbx(path, processors);
}