use std::fs::File;
use std::io::{Read};
use nom::error::{VerboseError, context};
use nom::{IResult, ToUsize, Offset};
use nom::bytes::complete::{tag, take};
use nom::number::complete::{le_u32, le_u8, le_i32, le_i16, le_f32, le_f64, le_i64};
use nom::sequence::tuple;
use nom::Err::Error;
use nom::multi::{count, length_data};
use byteorder::{ReadBytesExt, LittleEndian};

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
    end_offset: u32,
    num_properties: u32,
    property_list_len: u32,
    name: String,
    properties: Vec<PropertyRecordType>,
    nested_list: Vec<NodeRecord>,
}

struct Header<'a> {
    magic_string: &'a str,
    unknown_bytes: [u8; 2],
    version: u32,
}

type Res<T, U> = IResult<T, U, VerboseError<T>>;

fn parse_i16_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_i16(data)?;
    Ok((input, PropertyRecordType::SignedInt16(value)))
}

fn parse_i32_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_i32(data)?;
    Ok((input, PropertyRecordType::SignedInt32(value)))
}

fn parse_i64_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_i64(data)?;
    Ok((input, PropertyRecordType::SignedInt64(value)))
}

fn parse_f32_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_f32(data)?;
    Ok((input, PropertyRecordType::Float(value)))
}

fn parse_f64_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_f64(data)?;
    Ok((input, PropertyRecordType::Double(value)))
}

fn parse_bool_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, value) = le_u8(data)?;
    Ok((input, PropertyRecordType::Boolean(value == 1)))
}

struct ArrayMetaData {
    length: u32,
    encoding: u32,
    compressed_length: u32,
}

fn parse_array_metadata(data: &[u8]) -> Res<&[u8], ArrayMetaData> {
    tuple((le_u32, le_u32, le_u32))(data)
        .map(|(new_input, (length, encoding, compressed_length))| (new_input,
                                                                   ArrayMetaData {
                                                                       length,
                                                                       encoding,
                                                                       compressed_length,
                                                                   }))
}

fn parse_f32_array_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, metadata) = parse_array_metadata(data)?;
    if metadata.encoding == 0 {
        let (input, array) = count(le_f32, metadata.length as usize)(input)?;
        Ok((input, PropertyRecordType::FloatArray(array)))
    }
    else {
        let (input, deflated_data) = take(metadata.compressed_length)(input)?;
        let enflated_data = inflate::inflate_bytes_zlib(deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f32::<LittleEndian>().unwrap());
        }

        Ok((input, PropertyRecordType::FloatArray(array)))
    }
}

fn parse_f64_array_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, metadata) = parse_array_metadata(data)?;
    if metadata.encoding == 0 {
        let (input, array) = count(le_f64, metadata.length as usize)(input)?;
        Ok((input, PropertyRecordType::DoubleArray(array)))
    }
    else {
        let (input, deflated_data) = take(metadata.compressed_length)(input)?;
        let enflated_data = inflate::inflate_bytes_zlib(deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<f64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_f64::<LittleEndian>().unwrap());
        }

        Ok((input, PropertyRecordType::DoubleArray(array)))
    }
}

fn parse_i64_array_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, metadata) = parse_array_metadata(data)?;
    if metadata.encoding == 0 {
        let (input, array) = count(le_i64, metadata.length as usize)(input)?;
        Ok((input, PropertyRecordType::SignedInt64Array(array)))
    }
    else {
        let (input, deflated_data) = take(metadata.compressed_length)(input)?;
        let enflated_data = inflate::inflate_bytes_zlib(deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i64>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i64::<LittleEndian>().unwrap());
        }

        Ok((input, PropertyRecordType::SignedInt64Array(array)))
    }
}

fn parse_i32_array_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, metadata) = parse_array_metadata(data)?;
    if metadata.encoding == 0 {
        let (input, array) = count(le_i32, metadata.length as usize)(input)?;
        Ok((input, PropertyRecordType::SignedInt32Array(array)))
    }
    else {
        let (input, deflated_data) = take(metadata.compressed_length)(input)?;
        let enflated_data = inflate::inflate_bytes_zlib(deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_i32::<LittleEndian>().unwrap());
        }

        Ok((input, PropertyRecordType::SignedInt32Array(array)))
    }
}

fn parse_bool_array_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, metadata) = parse_array_metadata(data)?;
    if metadata.encoding == 0 {
        let (input, array) = count(le_i32, metadata.length as usize)(input)?;
        Ok((input, PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect())))
    }
    else {
        let (input, deflated_data) = take(metadata.compressed_length)(input)?;
        let enflated_data = inflate::inflate_bytes_zlib(deflated_data).unwrap();
        let length = enflated_data.len() / std::mem::size_of::<i32>();

        let mut array = Vec::with_capacity(length);
        for _ in 0..length {
            array.push(enflated_data.as_slice().read_u8().unwrap());
        }

        Ok((input, PropertyRecordType::BooleanArray(array.iter().map(|x| *x == 1).collect())))
    }
}

fn parse_string_property(data: &[u8]) -> Res<&[u8], PropertyRecordType> {
    let (input, bytes) = length_data(le_u32)(data)?;
    Ok((input, PropertyRecordType::String(String::from_utf8(Vec::from(bytes)).unwrap())))
}

fn parse_binary_data_property(data: &[u8]) -> Res<&[u8], PropertyRecordType> {
    let (input, bytes) = length_data(le_u32)(data)?;
    Ok((input, PropertyRecordType::BinaryData(Vec::from(bytes))))
}

fn parse_property(data: &[u8]) -> Res<&[u8], PropertyRecordType>
{
    let (input, type_code) = le_u8(data)?;

    match type_code as char {
        'Y' => parse_i16_property(input),
        'C' => parse_bool_property(data),
        'I' => parse_i32_property(input),
        'F' => parse_f32_property(input),
        'D' => parse_f64_property(input),
        'L' => parse_i64_property(input),
        'f' => parse_f32_array_property(input),
        'd' => parse_f64_array_property(input),
        'l' => parse_i64_array_property(input),
        'i' => parse_i32_array_property(input),
        'b' => parse_bool_array_property(input),
        'S' => parse_string_property(input),
        'R' => parse_binary_data_property(input),
        _ => panic!("WOooo")
    }
}

fn parse_properties<C>(count: C) -> impl Fn(&[u8]) -> Res<&[u8], Vec<PropertyRecordType>>
    where
        C: ToUsize,
{
    let c = count.to_usize();
    move |i: &[u8]| {
        let mut result = Vec::new();
        let mut input = i;
        for _ in 0..c {
            let (new_input, property) = parse_property(input)?;
            result.push(property);
            input = new_input;
        }

        Ok((input, result))
    }
}

fn parse_string(data: &[u8]) -> Res<&[u8], &str> {
    let (input, length) = le_u8(data)?;
    let (input, string_bytes) = take(length)(input)?;

    let foo = std::str::from_utf8(string_bytes);
    if foo.is_err() {
        let ff = 23232;
    }
    Ok((input, foo.unwrap()))
}

fn parse_node(data: &[u8], start_offset: usize, file_length: usize) -> Res<&[u8], Option<NodeRecord>> {
    let (input, end_offset) = le_u32(data)?;

    if end_offset == 0 {
        // End of file
        return Ok((input, None));
    }

    if end_offset as usize >= file_length {
        panic!("end offset is outside bounds");
    }

    let parse_num_props = le_u32;
    let parse_prop_list_len = le_u32;
    let parse_name = parse_string;

    let (input, (num_properties, property_length_bytes, name))
        = tuple((parse_num_props, parse_prop_list_len, parse_name))(input)?;

    if name == "Geometry" {
        let sdss = 22;
    }
    if name == "Vertices" {
        let sss = 22;
    }

    if name == "Model" {
        let sssss = 123;
    }

    let property_start_offset = input.as_ptr() as usize;
    if property_start_offset + property_length_bytes as usize > file_length {
        Error(nom::Err::Failure("property length out of bounds"));
    }

    let (mut input, properties) = parse_properties(num_properties as usize)(input)?;

    if property_length_bytes as usize != input.as_ptr() as usize - property_start_offset {
        Error(nom::Err::Failure("did not read correct amount of bytes when parsing properties."));
    }

    let mut child_nodes = Vec::new();
    if get_offset(start_offset, input) < end_offset as usize {
        let remaining_byte_count = end_offset as usize - get_offset(start_offset, input);
        let sentinel_block_length = std::mem::size_of::<u32>() * 3 + 1;
        if remaining_byte_count < sentinel_block_length {
            Error(nom::Err::Failure("insufficient amount of bytes at end of node"));
        }

        while get_offset(start_offset, input) < end_offset as usize - sentinel_block_length {
            let (new_input, node) = parse_node(input, start_offset, file_length)?;
            if node.is_some() {
                child_nodes.push(node.expect("Null node?"));
            }

            input = new_input;
        }

        let (new_input, sentinel_block) = count(le_u8, sentinel_block_length)(input)?;
        input = new_input;
        for i in 0..sentinel_block_length {
            if sentinel_block[i] != 0 {
                Error(nom::Err::Failure("sentinel block contains non-zero values"));
            }
        }
        //let (input, _) = parse_sentinel_block(input, sentinel_block_length)?;
        // let (input, nested_list) = parse_nodes(input)?;
    }

    if get_offset(start_offset, input)  != end_offset as usize {
        Error(nom::Err::Failure("end offset not reached."));
    }

    Ok((input, Some(NodeRecord {
        end_offset,
        property_list_len: property_length_bytes,
        num_properties,
        properties,
        nested_list: child_nodes,
        name: name.to_string(),
    })))
}

fn get_offset(start_offset: usize, end_offset: &[u8]) -> usize {
    end_offset.as_ptr() as usize - start_offset
}

fn magic_string(data: &[u8]) -> Res<&[u8], &str> {
    context(
        "header",
        tag("Kaydara FBX Binary  \0"),
    )(data).map(|(next_input, res)| (next_input, std::str::from_utf8(res).unwrap()))
}

fn header(data: &[u8]) -> Res<&[u8], Header> {
    let unknown_byte_parser = take(2usize);
    let version_parser = le_u32;

    let mut combined = context("header", tuple((magic_string, unknown_byte_parser, version_parser)));
    let (input, (magic, unknown_bytes, version)) = combined(data)?;

    Ok(
        (input,
         Header {
             magic_string: magic,
             unknown_bytes: [unknown_bytes[0], unknown_bytes[1]],
             version,
         })
    )
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
        PropertyRecordType::SignedInt64Array(x) => { println!("[i64]");}
        PropertyRecordType::SignedInt32Array(x) => { println!("[i32]");}
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

fn parse_nodes(data: &[u8], start_offset: usize, file_length: usize) -> Res<&[u8], Vec<NodeRecord>> {
    let mut input = data;
    let mut result = Vec::new();
    let end = input.as_ptr() as usize + file_length;
    while (input.as_ptr() as usize)  < end {
        let (new_input, root) = parse_node(input, start_offset, file_length).unwrap();
        input = new_input;

        if root.is_some() {
            result.push(root.unwrap());
        }
    }

    Ok((input, result))
}

fn main() {
    let mut file = File::open("/Users/emil/Downloads/untitled.fbx")
        .expect("Could not open file");

    // let mut reader = BufReader::new(file);
    let mut data = vec![];
    file.read_to_end(&mut data).unwrap();
    let (input, _header) = header(&data).unwrap();

    let (input, nodes) =
        parse_nodes(
            input,
            data.as_ptr() as usize,
            data.len()).unwrap();

    for node in &nodes {
        print_node(node, 0);
        println!();
    }

    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::{VerboseErrorKind, ErrorKind};

    #[test]
    fn magic_string_should_handle_happy_case() {
        let input = "Kaydara FBX Binary  \0".as_bytes();
        assert_eq!(magic_string(input), Ok(("".as_bytes(), "Kaydara FBX Binary  \0")));
    }

    #[test]
    fn magic_string_should_fail_on_wrong_case() {
        let input = "kaydara fbx BINARY  \0".as_bytes();
        assert_eq!(magic_string(input), Err(nom::Err::Error(VerboseError {
            errors: vec![
                ("kaydara fbx BINARY  \0".as_bytes(), VerboseErrorKind::Nom(ErrorKind::Tag)),
                ("kaydara fbx BINARY  \0".as_bytes(), VerboseErrorKind::Context("header"))
            ]
        })));
    }
}
