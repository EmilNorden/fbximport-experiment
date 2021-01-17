#![feature(seek_convenience)]
#![feature(bufreader_seek_relative)]

use std::fs::File;
use std::io::{Read, BufReader, Seek, Error, SeekFrom};
use byteorder::{ReadBytesExt, LittleEndian};
use std::str::Utf8Error;
use nom::lib::std::string::FromUtf8Error;

#[derive(Debug, PartialEq)]
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
enum ParseError {
    ValidationError(String),
    FormatError,
    IOError(Error)
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

type ParseResult<'a, T> = Result<T, ParseError>;

fn parse_i16_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i16::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt16(value))
}

fn parse_i32_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i32::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt32(value))
}

fn parse_i64_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_i64::<LittleEndian>()?;
    Ok(PropertyRecordType::SignedInt64(value))
}

fn parse_f32_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_f32::<LittleEndian>()?;
    Ok(PropertyRecordType::Float(value))
}

fn parse_f64_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_f64::<LittleEndian>()?;
    Ok(PropertyRecordType::Double(value))
}

fn parse_bool_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let value = reader.read_u8()?;
    Ok(PropertyRecordType::Boolean(value == 1))
}

struct ArrayMetaData {
    length: u32,
    encoding: u32,
    compressed_length: u32,
}

fn parse_array_metadata(reader: &mut dyn Read) -> ParseResult<ArrayMetaData> {
    let length = reader.read_u32::<LittleEndian>()?;
    let encoding = reader.read_u32::<LittleEndian>()?;
    let compressed_length = reader.read_u32::<LittleEndian>()?;

    Ok(ArrayMetaData {
        length,
        encoding,
        compressed_length
    })
}

fn parse_f32_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
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
        reader.read_exact(&mut deflated_data)?;
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f32::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::FloatArray(array))
    }
}

fn parse_f64_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
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
        reader.read_exact(&mut deflated_data)?;
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f64::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::DoubleArray(array))
    }
}

fn parse_i64_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
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
        reader.read_exact(&mut deflated_data)?;
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i64::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::SignedInt64Array(array))
    }
}

fn parse_i32_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
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
        reader.read_exact(&mut deflated_data)?;
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i32::<LittleEndian>().unwrap());
        }

        Ok(PropertyRecordType::SignedInt32Array(array))
    }
}

fn parse_bool_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let mut array = vec![0u8; metadata.length as usize];
        reader.read_exact(&mut array)?;
        Ok(PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect()))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data)?;
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_u8().unwrap());
        }

        Ok(PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect()))
    }
}

fn parse_string_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes)?;

    Ok(PropertyRecordType::String(String::from_utf8(Vec::from(bytes)).unwrap()))
}

fn parse_binary_data_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes)?;
    Ok(PropertyRecordType::BinaryData(bytes))
}

fn parse_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
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

fn parse_properties(reader: &mut dyn Read, num_properties: usize) -> ParseResult<Vec<PropertyRecordType>>
{
    let mut result = Vec::new();
    for _ in 0..num_properties {
        let property = parse_property(reader)?;
        result.push(property);
    }

    Ok(result)
}

fn parse_string(reader: &mut dyn Read) -> ParseResult<String> {
    let length = reader.read_u8()? as usize;
    let mut string_bytes = vec![0u8; length];
    reader.read_exact(&mut string_bytes)?;

    Ok(String::from_utf8(string_bytes)?)
}

fn parse_node<R>(reader: &mut R, file_length: usize) -> ParseResult<Option<NodeRecord>>
where
    R: Read + Seek{
    let end_offset = reader.read_u32::<LittleEndian>()? as usize;
    if end_offset == 0 {
        // End of file
        return Ok(None);
    }

    if end_offset >= file_length {
        return Err(ParseError::ValidationError("end offset is outside bounds".to_string()));
    }

    let num_properties = reader.read_u32::<LittleEndian>()?;
    let property_length_bytes = reader.read_u32::<LittleEndian>()?;
    let name = parse_string(reader)?;

    let property_start_offset = reader.stream_position()? as usize;
    if property_start_offset + property_length_bytes as usize > file_length {
        return Err(ParseError::ValidationError("property length out of bounds".to_string()));
    }
    let properties = parse_properties(reader,num_properties as usize)?;

    if property_length_bytes as usize != reader.stream_position()? as usize - property_start_offset {
        return Err(ParseError::ValidationError("did not read correct amount of bytes when parsing properties".to_string()));
    }

    let mut child_nodes = Vec::new();
    if (reader.stream_position()? as usize) < end_offset {
        let remaining_byte_count = end_offset - reader.stream_position()? as usize;
        let sentinel_block_length = std::mem::size_of::<u32>() * 3 + 1;
        if remaining_byte_count < sentinel_block_length {
            return Err(ParseError::ValidationError("insufficient amount of bytes at end of node".to_string()))
        }

        while (reader.stream_position()? as usize) < end_offset - sentinel_block_length {
            let node = parse_node(reader, file_length)?;
            if node.is_some() {
                child_nodes.push(node.expect("Null node?"));
            }
        }

        let mut sentinel_block = vec![0u8; sentinel_block_length];
        reader.read_exact(&mut sentinel_block)?;
        for i in 0..sentinel_block_length {
            if sentinel_block[i] != 0 {
                return Err(ParseError::ValidationError("sentinel block contains non-zero values".to_string()));
            }
        }
    }

    if reader.stream_position()? as usize != end_offset {
        return Err(ParseError::ValidationError("end offset not reached.".to_string()));
    }

    Ok(Some(NodeRecord {
        properties,
        nested_list: child_nodes,
        name: name.to_string(),
    }))
}

fn header<R>(reader: &mut R) -> ParseResult<Header>
where
    R: Read + Seek
{
    let mut magic_string_bytes = vec![0u8; 21];
    reader.read_exact(&mut magic_string_bytes)?;
    if std::str::from_utf8(&magic_string_bytes)? != "Kaydara FBX Binary  \0" {
        return Err(ParseError::ValidationError("file header magic string is incorrect".to_string()))
    }
    // Skip past unknown bytes
    reader.seek(SeekFrom::Current(2))?;
    // reader.seek_relative(2)?;

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
        PropertyRecordType::FloatArray(_) => { println!("[f32]"); }
        PropertyRecordType::DoubleArray(_) => { println!("[f64]"); }
        PropertyRecordType::SignedInt64Array(_) => { println!("[i64]"); }
        PropertyRecordType::SignedInt32Array(_) => { println!("[i32]"); }
        PropertyRecordType::BooleanArray(_) => { println!("[bool]"); }
        PropertyRecordType::String(x) => { println!("str: {}", x); }
        PropertyRecordType::BinaryData(_) => { println!("raw"); }
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

fn parse_nodes<R>(reader: &mut R, file_length: usize) -> ParseResult<Vec<NodeRecord>>
where
    R: Read + Seek
{
    let mut result = Vec::new();

    while (reader.stream_position()? as usize) < file_length {
        match parse_node(reader, file_length)? {
            Some(node) => result.push(node),
            None => break
        }
    }

    Ok(result)
}

fn main() {
    let file = File::open("/Users/emil/Downloads/untitled.fbx")
        .expect("Could not open file");

    let mut reader = BufReader::new(file);
    let length = reader.stream_len().unwrap() as usize;
    let _header = header(&mut reader).unwrap();

    let nodes =
        parse_nodes(
            &mut reader,
            length).unwrap();

    for node in &nodes {
        print_node(node, 0);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_i16_property_should_read_2_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4]);

        parse_i16_property(&mut input);

        assert_eq!(input.position(), 2);
    }

    #[test]
    fn parse_i16_property_should_return_correct_value() {
        let mut input = Cursor::new(vec![1u8, 2]);

        let value = parse_i16_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::SignedInt16(513));
    }

    #[test]
    fn parse_i16_property_should_return_error_if_not_enough_bytes() {
        let mut input = Cursor::new(vec![1u8]);

        let result = parse_i16_property(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn parse_i32_property_should_read_4_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8]);

        parse_i32_property(&mut input);

        assert_eq!(input.position(), 4);
    }

    #[test]
    fn parse_i32_property_should_return_correct_value() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4]);

        let value = parse_i32_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::SignedInt32(67305985));
    }

    #[test]
    fn parse_i32_property_should_return_error_if_not_enough_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3]);

        let result = parse_i32_property(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn parse_i64_property_should_read_8_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        parse_i64_property(&mut input);

        assert_eq!(input.position(), 8);
    }

    #[test]
    fn parse_i64_property_should_return_correct_value() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8]);

        let value = parse_i64_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::SignedInt64(578437695752307201));
    }

    #[test]
    fn parse_i64_property_should_return_error_if_not_enough_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7]);

        let result = parse_i64_property(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn parse_f32_property_should_read_4_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6]);

        parse_f32_property(&mut input);

        assert_eq!(input.position(), 4);
    }

    #[test]
    fn parse_f32_property_should_return_correct_value() {
        let mut input = Cursor::new(vec![10u8, 20, 30, 40]);

        let value = parse_f32_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::Float(0.00000000000000877510717));
    }

    #[test]
    fn parse_f32_property_should_return_error_if_not_enough_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3]);

        let result = parse_f32_property(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn parse_f64_property_should_read_8_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        parse_f64_property(&mut input);

        assert_eq!(input.position(), 8);
    }

    #[test]
    fn parse_f64_property_should_return_correct_value() {
        let mut input = Cursor::new(vec![10u8, 20, 30, 40, 50, 60, 70, 80]);

        let value = parse_f32_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::Float(0.00000000000000877510717));
    }

    #[test]
    fn parse_f64_property_should_return_error_if_not_enough_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4, 5, 6, 7]);

        let result = parse_f64_property(&mut input);

        assert!(result.is_err());
    }

    #[test]
    fn parse_bool_property_should_read_1_byte() {
        let mut input = Cursor::new(vec![1u8, 0]);

        parse_bool_property(&mut input);

        assert_eq!(input.position(), 1);
    }

    #[test]
    fn parse_bool_property_should_return_true_if_byte_is_1() {
        let mut input = Cursor::new(vec![1u8]);

        let value = parse_bool_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::Boolean(true));
    }

    #[test]
    fn parse_bool_property_should_return_true_if_byte_is_0() {
        let mut input = Cursor::new(vec![0u8]);

        let value = parse_bool_property(&mut input).unwrap();

        assert_eq!(value, PropertyRecordType::Boolean(false));
    }

    #[test]
    fn parse_bool_property_should_return_error_if_not_enough_bytes() {
        let empty: [u8; 0] = [0; 0];
        let mut input = Cursor::new(empty);

        let result = parse_f64_property(&mut input);

        assert!(result.is_err());
    }
}
