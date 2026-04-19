use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Jpeg,
    Tiff,
    Png,
}

pub fn detect(data: &[u8]) -> Result<FileType> {
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Ok(FileType::Jpeg);
    }
    if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        return Ok(FileType::Tiff);
    }
    if data.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']) {
        return Ok(FileType::Png);
    }
    Err(Error::BadMagic)
}
