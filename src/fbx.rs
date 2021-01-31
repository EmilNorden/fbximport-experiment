use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::io::{Error, BufReader, Seek};
use std::fs::File;
use crate::fbx::property::PropertyRecordType;
use crate::fbx::node::{NodeRecord, parse_nodes};
use crate::fbx::header::parse_header;
use multimap::MultiMap;
use crate::fbx::node_collection::NodeCollection;
use crate::fbx::importer::import;
use crate::mesh_processor::MeshProcessor;
use crate::scene::Scene;

mod property;
mod node;
mod header;
mod importer;
mod node_collection;

#[derive(Debug)]
enum ParseError {
    ValidationError(String),
    FormatError,
    IOError(Error),
}

impl From<std::io::Error> for ParseError {
    fn from(e: Error) -> Self {
        ParseError::IOError(e)
    }
}

impl From<Utf8Error> for ParseError {
    fn from(_: Utf8Error) -> Self {
        ParseError::FormatError
    }
}

impl From<FromUtf8Error> for ParseError {
    fn from(_: FromUtf8Error) -> Self {
        ParseError::FormatError
    }
}

type ParseResult<T> = Result<T, ParseError>;

fn parse_fbx(path: &str) -> NodeCollection {
    let file = File::open(path)
        .expect("Could not open file");

    let mut reader = BufReader::new(file);
    let length = reader.stream_len().unwrap() as usize;
    let _header = parse_header(&mut reader).unwrap();

    parse_nodes(
        &mut reader,
        length).unwrap()
}

pub fn import_fbx(path: &str, mesh_processors: Vec<Box<dyn MeshProcessor>>) -> Option<Scene> {
    let nodes = parse_fbx(path);

    if let Some(mut scene) = import(nodes) {
        for mesh in &mut scene.meshes {
            for processor in &mesh_processors {
                processor.process(mesh);
            }
        }

        return Some(scene);
    }

    None
}

fn print_property(prop: &PropertyRecordType, indent: usize) {
    print!("{}", String::from_utf8(vec![' ' as u8; indent]).unwrap());
    match prop {
        PropertyRecordType::SignedInt16(x) => { println!("i16 {}", x); }
        PropertyRecordType::Boolean(x) => { println!("bool {}", x); }
        PropertyRecordType::SignedInt32(x) => { println!("i32 {}", x); }
        PropertyRecordType::Float(x) => { println!("println {}", x); }
        PropertyRecordType::Double(x) => { println!("f64 {}", x); }
        PropertyRecordType::SignedInt64(x) => { println!("i64 {}", x); }
        PropertyRecordType::FloatArray(_) => { println!("[f32]"); }
        PropertyRecordType::DoubleArray(_) => { println!("[f64]"); }
        PropertyRecordType::SignedInt64Array(_) => { println!("[i64]"); }
        PropertyRecordType::SignedInt32Array(_) => { println!("[i32]"); }
        PropertyRecordType::BooleanArray(_) => { println!("[bool]"); }
        PropertyRecordType::String(x) => { println!("str {}", x); }
        PropertyRecordType::BinaryData(_) => { println!("raw"); }
    }
}

fn print_node(node: &NodeRecord, indent: usize) {
    println!("{}{}", String::from_utf8(vec!['-' as u8; indent]).unwrap(), &node.name);
    for prop in &node.properties {
        print_property(prop, indent);
    }

    /*for child in &node.nested_list {
        print_node(child, indent + 1);
    }*/
}