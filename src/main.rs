use std::fs::File;
use std::io::{BufReader, Read, SeekFrom, Seek, Cursor};
use byteorder::{LittleEndian, ByteOrder, BigEndian, ReadBytesExt};

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

fn parse_node_record(reader: &mut BufReader<File>) -> Option<NodeRecord> {
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
    println!("=> {}", name);

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
            'F' => PropertyRecordType::Float(LittleEndian::read_f32(&read32( reader))),
            'D' => PropertyRecordType::Double(LittleEndian::read_f64(&read64( reader))),
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
        match parse_node_record(reader) {
            Some(node) => subnodes.push(node),
            None => break,
        }
    }

    println!("<= {}", name);

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
    }
    else if encoding == 0 {

        let array_size = element_size * array_length;
        let mut array = vec![0u8; array_size];
        reader.read_exact(&mut array);

        return Cursor::new(array);
    }

    panic!("Unknown encoding: {}", encoding);
}

fn main() {
    let file = File::open("/Users/emil/Downloads/untitled.fbx")
        .expect("Could not open file");

    let mut reader = BufReader::new(file);
    /*let mut header: [u8; 20] = [0; 20];
    reader.read_exact(&mut header);
    let headerstr = String::from_utf8(header.to_vec()).unwrap();
    let mut garbage: [u8; 2] = [0; 2];
    reader.read_exact(&mut garbage);
    // reader.seek(SeekFrom::Current(2)); // Skip past two unknown bytes (0x1A and 0x00)
    let version = LittleEndian::read_u32(&read32(&mut reader));*/

    let mut header = [0u8; 27];
    reader.read_exact(&mut header);

    let node = parse_node_record(&mut reader);


    println!("Hello, world!");
}
