#![feature(seek_convenience)]

use std::fs::File;
use std::io::{Read, BufReader, Seek, Error};
use byteorder::{ReadBytesExt, LittleEndian};
use std::str::Utf8Error;

enum PropertyRecordType {
    SignedInt16(i16),
    Boolean(bool),
    SignedInt32(i32),
    Float(f32),
    Double(f64),
    SignedInt64(i64),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    SignedInt64Array(Vec<i64>),
    SignedInt32Array(Vec<i32>),
    BooleanArray(Vec<bool>),
    String(String),
    BinaryData(Vec<u8>),
}

struct NodeRecord {
    name: String,
    properties: Vec<PropertyRecordType>,
    nested_list: Vec<NodeRecord>,
}

struct Header {
    version: u32,
}

#[derive(Debug)]
enum ParseError<'a> {
    ValidationError(&'a str),
    FormatError,
    IOError(Error)
}

impl From<std::io::Error> for ParseError<'_> {
    fn from(e: Error) -> Self {
        ParseError::IOError(e)
    }
}

impl From<Utf8Error> for ParseError<'_> {
    fn from(e: Utf8Error) -> Self {
        ParseError::FormatError
    }
}

type ParseResult<'a, T> = Result<T, ParseError<'a>>;

fn parse_i16_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i16::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt16(value))
}

fn parse_i32_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i32::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt32(value))
}

fn parse_i64_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i64::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt64(value))
}

fn parse_f32_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_f32::<LittleEndian>()?;
    Ok(PropertyRecordType::Float(value))
}

fn parse_f64_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_f64::<LittleEndian>()?;
    Ok(PropertyRecordType::Double(value))
}

fn parse_bool_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_u8()?;
    Ok(PropertyRecordType::Boolean(value == 1))
}

struct ArrayMetaData {
    length: u32,
    encoding: u32,
    compressed_length: u32,
}

fn parse_array_metadata(reader: &mut BufReader<File>) -> ParseResult<ArrayMetaData> {
    let length = reader.read_u32::<LittleEndian>()?;
    let encoding = reader.read_u32::<LittleEndian>()?;
    let compressed_length = reader.read_u32::<LittleEndian>()?;

    Ok(ArrayMetaData {
        length,
        encoding,
        compressed_length
    })
}

fn parse_f32_array_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0f32; metadata.length as usize];
        for _ in 0..metadata.length {
            array.push(reader.read_f32::<LittleEndian>()?);
        }
        Ok(PropertyRecordType::FloatArray(array))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f32::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::FloatArray(array))
    }
}

fn parse_f64_array_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0f64; metadata.length as usize];
        for _ in 0..metadata.length {
            array.push(reader.read_f64::<LittleEndian>()?);
        }
        Ok(PropertyRecordType::DoubleArray(array))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f64::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::DoubleArray(array))
    }
}

fn parse_i64_array_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0i64; metadata.length as usize];
        for _ in 0..metadata.length {
            array.push(reader.read_i64::<LittleEndian>()?);
        }
        Ok(PropertyRecordType::SignedInt64Array(array))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i64::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::SignedInt64Array(array))
    }
}

fn parse_i32_array_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0i32; metadata.length as usize];
        for _ in 0..metadata.length {
            array.push(reader.read_i32::<LittleEndian>()?);
        }
        Ok(PropertyRecordType::SignedInt32Array(array))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i32::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::SignedInt32Array(array))
    }
}

fn parse_bool_array_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0u8; metadata.length as usize];
        reader.read_exact(&mut array);
        Ok(PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect()))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_u8().unwrap());
        }

        Ok(PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect()))
    }
}

fn parse_string_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes);

    Ok(PropertyRecordType::String(String::from_utf8(Vec::from(bytes)).unwrap()))
}

fn parse_binary_data_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes);
    Ok(PropertyRecordType::BinaryData(bytes))
}

fn parse_property(reader: &mut BufReader<File>) -> ParseResult<PropertyRecordType>
{
    let type_code = reader.read_u8()?;

    match type_code as char {
        'Y' => parse_i16_property(reader),
        'C' => parse_bool_property(reader),
        'I' => parse_i32_property(reader),
        'F' => parse_f32_property(reader),
        'D' => parse_f64_property(reader),
        'L' => parse_i64_property(reader),
        'f' => parse_f32_array_property(reader),
        'd' => parse_f64_array_property(reader),
        'l' => parse_i64_array_property(reader),
        'i' => parse_i32_array_property(reader),
        'b' => parse_bool_array_property(reader),
        'S' => parse_string_property(reader),
        'R' => parse_binary_data_property(reader),
        _ => panic!("WOooo")
    }
}

fn parse_properties(reader: &mut BufReader<File>, num_properties: usize) -> ParseResult<Vec<PropertyRecordType>>
{
    let mut result = Vec::new();
    for _ in 0..num_properties {
        let property = parse_property(reader)?;
        result.push(property);
    }

    Ok(result)
}

fn parse_string(reader: &mut BufReader<File>) -> ParseResult<&str> {
    let length = reader.read_u8()? as usize;
    let mut string_bytes = vec![0u8; length];
    reader.read_exact(&mut string_bytes);

    let foo = std::str::from_utf8(&string_bytes);
    if foo.is_err() {
        let ff = 23232;
    }
    Ok(foo.unwrap())
}

fn parse_node(reader: &mut BufReader<File>, start_offset: usize, file_length: usize) -> ParseResult<Option<NodeRecord>> {
    let end_offset = reader.read_u32::<LittleEndian>()? as usize;
    if end_offset == 0 {
        // End of file
        return Ok(None);
    }

    if end_offset >= file_length {
        return Err(ParseError::ValidationError("end offset is outside bounds"));
    }

    let num_properties = reader.read_u32::<LittleEndian>()?;
    let property_length_bytes = reader.read_u32::<LittleEndian>()?;
    let name = parse_string(reader)?;

    if name == "Geometry" {
        let sdss = 22;
    }
    if name == "Vertices" {
        let sss = 22;
    }

    if name == "Model" {
        let sssss = 123;
    }

    let property_start_offset = reader.stream_position()? as usize;
    if property_start_offset + property_length_bytes as usize > file_length {
        return Err(ParseError::ValidationError("property length out of bounds"));
    }

    let properties = parse_properties(reader,num_properties as usize)?;

    if property_length_bytes as usize != reader.stream_position()? as usize - property_start_offset {
        return Err(ParseError::ValidationError("did not read correct amount of bytes when parsing properties"));
    }

    let mut child_nodes = Vec::new();
    if (reader.stream_position()? as usize) < end_offset {
        let remaining_byte_count = end_offset - reader.stream_position()? as usize;
        let sentinel_block_length = std::mem::size_of::<u32>() * 3 + 1;
        if remaining_byte_count < sentinel_block_length {
            return Err(ParseError::ValidationError("insufficient amount of bytes at end of node"))
        }

        while (reader.stream_position()? as usize) < end_offset - sentinel_block_length {
            let node = parse_node(reader, start_offset, file_length)?;
            if node.is_some() {
                child_nodes.push(node.expect("Null node?"));
            }
        }

        let mut sentinel_block = vec![0u8; sentinel_block_length];
        reader.read_exact(&mut sentinel_block);
        for i in 0..sentinel_block_length {
            if sentinel_block[i] != 0 {
                return Err(ParseError::ValidationError("sentinel block contains non-zero values"));
            }
        }
    }

    if reader.stream_position()? as usize != end_offset {
        return Err(ParseError::ValidationError("end offset not reached."));
    }

    Ok(Some(NodeRecord {
        properties,
        nested_list: child_nodes,
        name: name.to_string(),
    }))
}

fn get_offset(start_offset: usize, end_offset: &[u8]) -> usize {
    end_offset.as_ptr() as usize - start_offset
}


fn header(reader: &mut BufReader<File>) -> ParseResult<Header> {
    let mut magic_string = vec![0u8; 21];
    reader.read_exact(&mut magic_string);

    if std::str::from_utf8(&magic_string)? != "Kaydara FBX Binary  " {
        return Err(ParseError::ValidationError("file header magic string is incorrect"))
    }

    // read and throw away two bytes
    reader.read_u8();
    reader.read_u8();

    let version = reader.read_u32::<LittleEndian>()?;

    Ok(Header { version })
}

fn print_property(prop: &PropertyRecordType, indent: usize) {
    print!("{}", String::from_utf8(vec![' ' as u8; indent]).unwrap());
    match prop {
        PropertyRecordType::SignedInt16(x) => { println!("i16: {}", x); }
        PropertyRecordType::Boolean(x) => { println!("bool: {}", x); }
        PropertyRecordType::SignedInt32(x) => { println!("i32: {}", x); }
        PropertyRecordType::Float(x) => { println!("f32: {}", x); }
        PropertyRecordType::Double(x) => { println!("f64: {}", x); }
        PropertyRecordType::SignedInt64(x) => { println!("i64: {}", x); }
        PropertyRecordType::FloatArray(x) => { println!("[f32]"); }
        PropertyRecordType::DoubleArray(x) => { println!("[f64]"); }
        PropertyRecordType::SignedInt64Array(x) => { println!("[i64]"); }
        PropertyRecordType::SignedInt32Array(x) => { println!("[i32]"); }
        PropertyRecordType::BooleanArray(x) => { println!("[bool]"); }
        PropertyRecordType::String(x) => { println!("str: {}", x); }
        PropertyRecordType::BinaryData(x) => { println!("raw"); }
    }
}

fn print_node(node: &NodeRecord, indent: usize) {
    println!("{}{}", String::from_utf8(vec!['-' as u8; indent]).unwrap(), &node.name);

    for prop in &node.properties {
        print_property(prop, indent);
    }

    for child in &node.nested_list {
        print_node(child, indent + 1);
    }
}

fn parse_nodes(reader: &mut BufReader<File>, start_offset: usize, file_length: usize) -> ParseResult<Vec<NodeRecord>> {
    let mut result = Vec::new();
    while (reader.stream_position()? as usize) < file_length {
        let root = parse_node(reader, start_offset, file_length)?;
        if root.is_some() {
            result.push(root.unwrap());
        }
    }

    Ok(result)
}

fn main() {
    let mut file = File::open("/Users/emil/Downloads/untitled.fbx")
        .expect("Could not open file");

    let mut reader = BufReader::new(file);
    let length = reader.stream_len().unwrap() as usize;
    let header = header(&mut reader).unwrap();

    let nodes =
        parse_nodes(
            &mut reader,
            0,
            length).unwrap();

    for node in &nodes {
        print_node(node, 0);
        println!();
    }

    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
}
