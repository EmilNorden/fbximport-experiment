#![feature(seek_convenience)]
#![feature(bufreader_seek_relative)]

use crate::fbx::import_fbx;

mod fbx;

fn main() {
    let path = "/Users/emil/Downloads/untitled.fbx";

    let model = import_fbx(path);
    println!("nodes: {}", model);
}