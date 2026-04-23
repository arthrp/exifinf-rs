use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    Jpeg,
    Tiff,
    Png,
    /// ISO base media / QuickTime (MOV, MP4, M4A, M4V, HEIC, 3GP, etc.)
    Qt,
}

/// Top-level BMFF fourccs that we treat as QuickTime/MP4 family.
fn is_bmff_top_type(t: &[u8; 4]) -> bool {
    matches!(
        t,
        b"ftyp" | b"moov" | b"mdat" | b"wide" | b"free" | b"skip" | b"metx" | b"styp" | b"sidx" | b"moof"
    )
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
    if data.len() >= 8 {
        if &data[4..8] == b"ftyp" {
            return Ok(FileType::Qt);
        }
        let mut typ = [0u8; 4];
        typ.copy_from_slice(&data[4..8]);
        if is_bmff_top_type(&typ) {
            return Ok(FileType::Qt);
        }
    }
    Err(Error::BadMagic)
}
