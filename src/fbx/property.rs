use crate::fbx::ParseResult;
use std::io::Read;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, PartialEq)]
pub enum PropertyRecordType {
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
        other => panic!("Unexpected type_code: {}", other)
    }
}

pub(super) fn parse_properties(reader: &mut dyn Read, num_properties: usize) -> ParseResult<Vec<PropertyRecordType>>
{
    let mut result = Vec::new();
    for _ in 0..num_properties {
        let property = parse_property(reader)?;
        result.push(property);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_i16_property_should_read_2_bytes() {
        let mut input = Cursor::new(vec![1u8, 2, 3, 4]);

        parse_i16_property(&mut input).unwrap();

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

        parse_i32_property(&mut input).unwrap();

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

        parse_i64_property(&mut input).unwrap();

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

        parse_f32_property(&mut input).unwrap();

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

        parse_f64_property(&mut input).unwrap();

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

        parse_bool_property(&mut input).unwrap();

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
