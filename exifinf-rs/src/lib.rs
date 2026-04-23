//! Read-only EXIF extraction for JPEG, TIFF, PNG, and QuickTime/MP4/HEIC (BMFF) metadata (subset of ExifTool semantics).

mod byteorder;
mod detect;
mod error;
mod format;
mod gps;
mod jpeg;
mod metadata;
mod png;
mod printconv;
#[allow(dead_code)]
mod tables;
mod tag_def;
mod tiff;
mod value;
mod iso6709;
mod qt;
mod qt_tags;

pub use error::{Error, Result};
pub use metadata::{Metadata, TagRecord};
pub use printconv::format_record;
pub use value::Value;

use std::path::Path;

use crate::detect::FileType;

pub fn extract(bytes: &[u8]) -> Result<Metadata> {
    let mut meta = Metadata::default();
    match detect::detect(bytes)? {
        FileType::Jpeg => jpeg::parse(&mut meta, bytes)?,
        FileType::Tiff => tiff::parse_exif_slice(&mut meta, bytes)?,
        FileType::Png => png::parse(&mut meta, bytes)?,
        FileType::Qt => qt::parse(&mut meta, bytes)?,
    }
    Ok(meta)
}

pub fn extract_from_path(path: &Path) -> Result<Metadata> {
    let bytes = std::fs::read(path)?;
    extract(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    fn minimal_tiff_image_description() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"II");
        v.extend_from_slice(&42u16.to_le_bytes());
        v.extend_from_slice(&8u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&0x010eu16.to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&5u32.to_le_bytes());
        v.extend_from_slice(&26u32.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(b"test\0");
        v
    }

    #[test]
    fn detect_jpeg() {
        let b = [0xff, 0xd8, 0xff, 0xe0];
        assert_eq!(detect::detect(&b).unwrap(), FileType::Jpeg);
    }

    #[test]
    fn detect_png() {
        let b = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        assert_eq!(detect::detect(&b).unwrap(), FileType::Png);
    }

    #[test]
    fn tiff_one_string_tag() {
        let m = extract(&minimal_tiff_image_description()).unwrap();
        let d = m
            .tags
            .iter()
            .find(|t| t.name == "ImageDescription")
            .unwrap();
        assert_eq!(d.value, Value::Ascii("test".into()));
    }

    #[test]
    fn detect_qt() {
        let mov = [0, 0, 0, 8, b'm', b'o', b'o', b'v'];
        assert_eq!(detect::detect(&mov).unwrap(), FileType::Qt);
        let ftyp = [0, 0, 0, 0x1c, b'f', b't', b'y', b'p'];
        assert_eq!(detect::detect(&ftyp).unwrap(), FileType::Qt);
    }

    #[test]
    fn jpeg_app1_exif_round_trip() {
        let tiff = minimal_tiff_image_description();
        let mut j = vec![0xff, 0xd8];
        let payload_len = (2 + 6 + tiff.len()) as u16;
        j.push(0xff);
        j.push(0xe1);
        j.extend_from_slice(&payload_len.to_be_bytes());
        j.extend_from_slice(b"Exif\0\0");
        j.extend_from_slice(&tiff);
        j.push(0xff);
        j.push(0xd9);
        let m = extract(&j).unwrap();
        assert!(m.tags.iter().any(|t| t.name == "ImageDescription"));
    }
}
