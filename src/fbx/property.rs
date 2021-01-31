use crate::fbx::ParseResult;
use std::io::{Read, Cursor, Seek};
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

fn get_property_raw_byte_cursor<T>(reader: &mut dyn Read) -> ParseResult<Cursor<Vec<u8>>> {
    let metadata = parse_array_metadata(reader)?;
    if metadata.encoding == 0 {
        let byte_count = std::mem::size_of::<T>() * metadata.length as usize;
        let mut array = vec![0u8; byte_count];
        reader.read_exact(&mut array)?;
        Ok(Cursor::new(array))
    } else {
        let mut deflated_data = vec![0u8; metadata.compressed_length as usize];
        reader.read_exact(&mut deflated_data)?;
        let inflated_data = inflate::inflate_bytes_zlib(&deflated_data).unwrap();
        Ok(Cursor::new(inflated_data))
    }
}

fn apply_transform_on_byte_stream<T>(input: &mut Cursor<Vec<u8>>, transform: &dyn Fn(&mut Cursor<Vec<u8>>) -> ParseResult<T>) -> ParseResult<Vec<T>> {
    let elements = input.stream_len()? as usize / std::mem::size_of::<T>();
    let mut array = Vec::with_capacity(elements);
    for _ in 0..elements {
        array.push(transform(input)?);
    }

    Ok(array)
}

fn parse_f32_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let mut cursor = get_property_raw_byte_cursor::<f32>(reader)?;
    let array = apply_transform_on_byte_stream(
        &mut cursor,
        &|x| Ok(x.read_f32::<LittleEndian>()?))?;

    Ok(PropertyRecordType::FloatArray(array))
}

fn parse_f64_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let mut cursor = get_property_raw_byte_cursor::<f64>(reader)?;
    let array = apply_transform_on_byte_stream(
        &mut cursor,
        &|x| Ok(x.read_f64::<LittleEndian>()?))?;

    Ok(PropertyRecordType::DoubleArray(array))
}

fn parse_i64_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let mut cursor = get_property_raw_byte_cursor::<i64>(reader)?;
    let array = apply_transform_on_byte_stream(
        &mut cursor,
        &|x| Ok(x.read_i64::<LittleEndian>()?))?;

    Ok(PropertyRecordType::SignedInt64Array(array))
}

fn parse_i32_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let mut cursor = get_property_raw_byte_cursor::<i32>(reader)?;
    let array = apply_transform_on_byte_stream(
        &mut cursor,
        &|x| Ok(x.read_i32::<LittleEndian>()?))?;

    Ok(PropertyRecordType::SignedInt32Array(array))
}

fn parse_bool_array_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType>
{
    let mut cursor = get_property_raw_byte_cursor::<bool>(reader)?;
    let array = apply_transform_on_byte_stream(
        &mut cursor,
        &|x| Ok(x.read_u8()? == 1))?;

    Ok(PropertyRecordType::BooleanArray(array))
}

fn parse_string_property(reader: &mut dyn Read) -> ParseResult<PropertyRecordType> {
    let length = reader.read_u32::<LittleEndian>()? as usize;
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes)?;

    // For some reason, the names of objects consists of [actual name][bytes 0 and 1][object type].
    // For now I will just parse everything up to the null byte, to avoid problems downstream.
    let actual_string_length = bytes.iter().position(|x| *x == 0).unwrap_or(bytes.len());

    let null_terminated_data = bytes[0..actual_string_length].to_vec();

    Ok(PropertyRecordType::String(String::from_utf8(null_terminated_data).unwrap()))
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
    use byteorder::WriteBytesExt;
    use deflate::deflate_bytes_zlib;

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

    fn fill_array_metadata(data: &mut Vec<u8>, length: u32, encoding: u32, compressed_length: u32) {
        data.write_u32::<LittleEndian>(length);
        data.write_u32::<LittleEndian>(encoding);
        data.write_u32::<LittleEndian>(compressed_length);
    }

    #[test]
    fn get_property_raw_byte_cursor_should_handle_uncompressed_data() {
        // Arrange
        let payload = vec![1u8, 2u8, 3u8, 4u8, 3u8, 2u8, 1u8, 0u8];
        let mut data = Vec::new();
        fill_array_metadata(&mut data, 2, 0, 0);
        data.append(&mut payload.clone());

        // Act
        let result = get_property_raw_byte_cursor::<i32>(&mut Cursor::new(data));

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), payload);
    }

    #[test]
    fn get_property_raw_byte_cursor_should_handle_compressed_data() {
        // Arrange
        // these are signed 32-bit values 0 1 2 deflated.
        let payload = vec![120, 156, 99, 0, 2, 70, 32, 102, 2, 98, 0, 0, 28, 0, 4];
        let mut data = Vec::new();
        fill_array_metadata(&mut data, 0, 1, payload.len() as u32);
        data.append(&mut payload.clone());

        // Act
        let result = get_property_raw_byte_cursor::<i32>(&mut Cursor::new(data));

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), vec![0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn get_property_raw_byte_cursor_should_return_error_if_not_enough_bytes() {
        // Arrange
        let mut data = Vec::new();
        fill_array_metadata(&mut data, 1, 0, 0);

        // Act
        let result = get_property_raw_byte_cursor::<i32>(&mut Cursor::new(data));

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn apply_transform_on_byte_stream_should_apply_transform() {
        // Arrange
        let data = vec![9, 0, 0, 0, 4, 0, 0, 0, 7, 1, 0, 0];

        // Act
        let result = apply_transform_on_byte_stream(
            &mut Cursor::new(data),
            &|x| Ok(x.read_i32::<LittleEndian>().unwrap() + 1));

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![10i32, 5, 264])
    }

    #[test]
    fn apply_transform_on_byte_stream_should_handle_empty_input() {
        // Arrange
        let data = Vec::<u8>::new();

        // Act
        let result = apply_transform_on_byte_stream(
            &mut Cursor::new(data),
            &|x| Ok(x.read_i32::<LittleEndian>().unwrap() + 1));

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn apply_transform_on_byte_stream_should_handle_incomplete_stream() {
        // Arrange
        // data only contains enough bytes for ONE i32, leaving 3 bytes
        let data = vec![9, 0, 0, 0, 8, 0, 0];
        let mut cursor = Cursor::new(data);

        // Act
        let result = apply_transform_on_byte_stream(
            &mut cursor,
            &|x| Ok(x.read_i32::<LittleEndian>().unwrap()));

        // Assert
        assert!(result.is_ok());
        let unwrapped_result = result.unwrap();
        assert_eq!(unwrapped_result.len(), 1);
        assert_eq!(unwrapped_result[0], 9);
        assert_eq!(cursor.position(), 4);
    }

    #[test]
    fn parse_i32_array_property_should_handle_uncompressed_data() {
        // Arrange
        let mut data = Vec::new();
        fill_array_metadata(&mut data, 5, 0, 0);
        for i in 0..5 {
            data.write_i32::<LittleEndian>(i).unwrap();
        }
        let mut input = Cursor::new(data);

        // Act
        let result = parse_i32_array_property(&mut input);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PropertyRecordType::SignedInt32Array(vec![0, 1, 2, 3, 4]));
    }
}
