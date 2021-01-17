use crate::fbx::{ParseError, ParseResult};
use std::io::{Read, Seek};
use byteorder::{ReadBytesExt, LittleEndian};
use crate::fbx::property::{PropertyRecordType, parse_properties};

pub struct NodeRecord {
    pub(crate) name: String,
    pub(crate) properties: Vec<PropertyRecordType>,
    pub(crate) nested_list: Vec<NodeRecord>,
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

pub(super) fn parse_nodes<R>(reader: &mut R, file_length: usize) -> ParseResult<Vec<NodeRecord>>
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