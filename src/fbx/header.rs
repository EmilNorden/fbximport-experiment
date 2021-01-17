use std::io::{Read, Seek, SeekFrom};
use crate::fbx::{ParseResult, ParseError};
use byteorder::{ReadBytesExt, LittleEndian};

pub struct Header {
    version: u32,
}

pub(super) fn parse_header<R>(reader: &mut R) -> ParseResult<Header>
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