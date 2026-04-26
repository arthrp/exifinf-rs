//! EXIF / metadata: read for JPEG, TIFF, PNG, and QuickTime/MP4/HEIC; strip for JPEG, PNG, QT.

mod byteorder;
mod detect;
mod error;
mod format;
mod gps;
mod iso6709;
mod jpeg;
mod jpeg_strip;
mod metadata;
mod png;
mod png_strip;
mod printconv;
mod qt;
mod qt_strip;
mod qt_tags;
#[allow(dead_code)]
mod tables;
mod tag_def;
mod tiff;
mod value;
mod common;

pub use error::{Error, Result};
pub use metadata::{Metadata, TagRecord};
pub use printconv::format_record;
pub use value::Value;

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::detect::FileType;

/// Options for `strip_metadata`, mirroring a subset of `exiftool -all=` and common keep-flags.
#[derive(Debug, Clone)]
pub struct StripOptions {
    /// Keep ICC profile: JPEG `APP2` ICC, PNG `iCCP`, and HEIC `colr` in `ipco` when set.
    pub keep_icc: bool,
    /// Keep color-related ancillaries: PNG `gAMA`, `cHRM`, `sRGB`, `tRNS`, `bKGD`, …
    pub keep_color_info: bool,
    /// Keep JPEG JFIF `APP0` segment.
    pub keep_jfif: bool,
    /// When set, do not write a sidecar `*_original` in `strip_metadata_in_place`.
    pub overwrite_original: bool,
}

impl Default for StripOptions {
    /// Match aggressive `exiftool -all=` (strip ICC, JFIF, and color aux).
    fn default() -> Self {
        Self {
            keep_icc: false,
            keep_color_info: false,
            keep_jfif: false,
            overwrite_original: false,
        }
    }
}

/// Remove metadata; returns new bytes. TIFF is not supported in this version.
pub fn strip_metadata(bytes: &[u8], opts: &StripOptions) -> Result<Vec<u8>> {
    match detect::detect(bytes)? {
        FileType::Jpeg => jpeg_strip::strip(bytes, opts),
        FileType::Png => png_strip::strip(bytes, opts),
        FileType::Qt => qt_strip::strip(bytes, opts),
        FileType::Tiff => Err(Error::Unsupported("TIFF strip")),
    }
}

/// Strip metadata in place: writes `<filename>_original` first (unless `overwrite_original`),
/// then rewrites the file using a same-directory temp and atomic rename.
pub fn strip_metadata_in_place(path: &Path, opts: &StripOptions) -> Result<()> {
    let data = fs::read(path)?;
    let out = strip_metadata(&data, opts)?;
    if !opts.overwrite_original {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let sidecar = path.with_file_name(format!("{name}_original"));
            if !sidecar.exists() {
                fs::copy(path, &sidecar)?;
            }
        }
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".exifinf_strip_{:x}.tmp", t));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&out)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

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
