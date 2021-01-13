use std::fs::File;
use std::io::{BufReader, Read, SeekFrom, Seek, Cursor};
use byteorder::{LittleEndian, ByteOrder, BigEndian, ReadBytesExt};
use nom::error::{VerboseError, context, FromExternalError, ParseError, ErrorKind};
use nom::{IResult, InputIter, ToUsize, Slice, Parser, InputLength};
use nom::bytes::complete::{tag, take};
use nom::number::complete::{le_u32, le_u8, le_i32, le_i16, le_f32, le_f64, le_i64};
use nom::sequence::tuple;
use nom::lib::std::str::Utf8Error;
use std::ops::RangeFrom;
use nom::Err::Error;
use nom::multi::{count, length_data};

fn read_boolean(reader: &mut BufReader<File>) -> bool {
    reader.read_u8().unwrap() == 0b00000001
}

fn read_bytes(reader: &mut BufReader<File>, count: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; count];
    reader.read_exact(&mut buffer);

    buffer
}

fn read64(reader: &mut BufReader<File>) -> [u8; 8] {
    let mut buffer: [u8; 8] = [0; 8];
    reader.read_exact(&mut buffer);

    buffer
}

fn read32(reader: &mut BufReader<File>) -> [u8; 4] {
    let mut buffer: [u8; 4] = [0; 4];
    reader.read_exact(&mut buffer);

    buffer
}

fn read16(reader: &mut BufReader<File>) -> [u8; 2] {
    let mut buffer: [u8; 2] = [0; 2];
    reader.read_exact(&mut buffer);

    buffer
}


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

        let (input, array) = count(le_f32, length)(input)?;
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

        let (input, array) = count(le_f64, length)(input)?;
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

        let (input, array) = count(le_i64, length)(input)?;
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

        let (input, array) = count(le_i32, length)(input)?;
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

        let (input, array) = count(le_i32, length)(input)?;
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
        'Y' => parse_i32_property(input),
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

pub fn parse_properties<C>(count: C) -> impl Fn(&[u8]) -> Res<&[u8], Vec<PropertyRecordType>>
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

    Ok((input, std::str::from_utf8(string_bytes).unwrap()))
}

fn parse_nodes(data: &[u8]) -> Res<&[u8], Vec<NodeRecord>> {
    Ok((data, Vec::<NodeRecord>::new()))
}

fn parse_sentinel_block(data: &[u8], block_length: usize) -> Res<&[u8], ()> {
    context(
        "sentinel",
        tag(vec![0u8; block_length].as_slice()),
    )(data).map(|(next_input, tag)| (next_input, ()))
}

fn parse_node(data: &[u8], start_offset: usize, file_length: usize) -> Res<&[u8], Option<NodeRecord>> {
    let (input, end_offset) = le_u32(data)?;

    if end_offset == 0 {
        // End of file
        return Ok((input, None));
    }

    if end_offset as usize >= file_length {
        Error(nom::Err::Failure("end offset is outside bounds"));
    }

    let parse_num_props = le_u32;
    let parse_prop_list_len = le_u32;
    let parse_name = parse_string;

    let (input, (num_properties, property_length_bytes, name))
        = tuple((parse_num_props, parse_prop_list_len, parse_name))(input)?;

    let property_start_offset = input.as_ptr() as usize;
    let parse_properties = parse_properties(num_properties as usize);

    if property_start_offset + property_length_bytes as usize > file_length {
        Error(nom::Err::Failure("property length out of bounds"));
    }

    let (mut input, properties) = parse_properties(input)?;

    if property_length_bytes as usize != input.as_ptr() as usize - property_start_offset {
        Error(nom::Err::Failure("did not read correct amount of bytes when parsing properties."));
    }

    let mut child_nodes = Vec::new();
    let offset = get_offset(start_offset, input);
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
            else {
                let f = 232;
            }


            input = new_input;
        }

        let (input, sentinel_block) = count(le_u8, sentinel_block_length)(input)?;
        for i in 0..sentinel_block_length {
            if sentinel_block[i] != 0 {
                Error(nom::Err::Failure("sentinel block contains non-zero values"));
            }
        }
        //let (input, _) = parse_sentinel_block(input, sentinel_block_length)?;
        // let (input, nested_list) = parse_nodes(input)?;
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

fn main() {
    let mut file = File::open("/Users/emil/Downloads/untitled.fbx")
        .expect("Could not open file");

    // let mut reader = BufReader::new(file);
    let mut data = vec![];
    file.read_to_end(&mut data);
    let (input, header) = header(&data).unwrap();

    let root = parse_node(input, data.as_ptr() as usize, data.len());


    println!("Hello, world!");
}


fn parse_node_record(reader: &mut BufReader<File>, indent: usize) -> Option<NodeRecord> {
    let end_offset = LittleEndian::read_u32(&read_bytes(reader, 4));
    let num_properties = LittleEndian::read_u32(&read_bytes(reader, 4));
    let property_list_len = LittleEndian::read_u32(&read_bytes(reader, 4));
    let name_len = reader.read_u8().unwrap();

    if end_offset == 0 && num_properties == 0 && property_list_len == 0 && name_len == 0 {
        // This is a null record
        return None;
    }

    let mut name_buf = vec![0u8; name_len as usize];
    reader.read_exact(&mut name_buf);
    let name = String::from_utf8(name_buf).unwrap();
    let indent_chars = vec!['-' as u8; indent];
    println!("{}{}", String::from_utf8(indent_chars).unwrap(), name);

    if name.eq("ReferenceTime") {
        let f = 232;
    }

    let mut properties = Vec::new();
    for i in 0..num_properties {
        let record_type = reader.read_u8().unwrap() as char;
        let data = match record_type {
            'Y' => PropertyRecordType::SignedInt16(LittleEndian::read_i16(&read16(reader))),
            'C' => PropertyRecordType::Boolean(reader.read_u8().unwrap() == 0b00000001),
            'I' => PropertyRecordType::SignedInt32(LittleEndian::read_i32(&read32(reader))),
            'F' => PropertyRecordType::Float(LittleEndian::read_f32(&read32(reader))),
            'D' => PropertyRecordType::Double(LittleEndian::read_f64(&read64(reader))),
            'L' => PropertyRecordType::SignedInt64(LittleEndian::read_i64(&read64(reader))),
            'f' => {
                let mut data = read_array_data(reader, 4);
                let byte_count = data.get_ref().len();
                let mut array = Vec::with_capacity(byte_count / 4);
                while (data.position() as usize) < byte_count - 1 {
                    array.push(data.read_f32::<LittleEndian>().unwrap());
                }
                PropertyRecordType::FloatArray(array)
            }
            'd' => {
                let mut data = read_array_data(reader, 8);
                let byte_count = data.get_ref().len();
                let mut array = Vec::with_capacity(byte_count / 8);
                while (data.position() as usize) < byte_count - 1 {
                    array.push(data.read_f64::<LittleEndian>().unwrap());
                }

                PropertyRecordType::DoubleArray(array)
            }
            'l' => {
                let mut data = read_array_data(reader, 8);
                let byte_count = data.get_ref().len();
                let mut array = Vec::with_capacity(byte_count / 8);
                while (data.position() as usize) < byte_count - 1 {
                    array.push(data.read_i64::<LittleEndian>().unwrap());
                }

                PropertyRecordType::SignedInt64Array(array)
            }
            'i' => {
                let mut data = read_array_data(reader, 4);
                let byte_count = data.get_ref().len();
                let mut array = Vec::with_capacity(byte_count / 4);
                while (data.position() as usize) < byte_count - 1 {
                    array.push(data.read_i32::<LittleEndian>().unwrap());
                }

                PropertyRecordType::SignedInt32Array(array)
            }
            'b' => {
                let mut data = read_array_data(reader, 1);
                let byte_count = data.get_ref().len();
                let mut array = Vec::with_capacity(byte_count);
                while (data.position() as usize) < byte_count - 1 {
                    array.push(data.read_u8().unwrap() == 0b00000001);
                }

                PropertyRecordType::BooleanArray(array)
            }
            'S' => {
                let length = LittleEndian::read_u32(&read32(reader)) as usize;
                let mut buf = vec![0u8; length];
                reader.read_exact(&mut buf);

                PropertyRecordType::String(String::from_utf8(buf).unwrap())
            }
            'R' => {
                let length = LittleEndian::read_u32(&read32(reader)) as usize;
                let mut buf = vec![0u8; length];
                reader.read_exact(&mut buf);

                PropertyRecordType::BinaryData(buf)
            }
            _ => panic!("Unknown property record type {}", record_type),
        };

        properties.push(data);
    }

    let mut subnodes = Vec::new();
    loop {
        match parse_node_record(reader, indent + 1) {
            Some(node) => subnodes.push(node),
            None => break,
        }
    }

    Some(NodeRecord {
        properties,
        num_properties,
        end_offset,
        name,
        nested_list: subnodes,
        property_list_len,
    })
}

fn read_array_data(reader: &mut BufReader<File>, element_size: usize) -> Cursor<Vec<u8>> {
    let array_length = reader.read_u32::<LittleEndian>().unwrap() as usize;
    let encoding = reader.read_u32::<LittleEndian>().unwrap();
    let compressed_length = reader.read_u32::<LittleEndian>().unwrap() as usize;
    if encoding == 1 {
        let mut deflated_data = vec![0u8; compressed_length];
        reader.read_exact(&mut deflated_data);
        let enflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();

        return Cursor::new(enflated_data);
    } else if encoding == 0 {
        let array_size = element_size * array_length;
        let mut array = vec![0u8; array_size];
        reader.read_exact(&mut array);

        return Cursor::new(array);
    }

    panic!("Unknown encoding: {}", encoding);
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
