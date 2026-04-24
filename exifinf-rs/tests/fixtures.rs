//! Integration checks against sample images in the workspace `img/` directory.

use std::path::PathBuf;

use std::fs;

use exifinf_rs::{extract, extract_from_path, strip_metadata, StripOptions};

fn images_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../img")
}

#[test]
fn extract_jpeg_exif_works() {
    let p = images_dir().join("ExifTool.jpg");
    let m = extract_from_path(&p).expect("extract");
    assert!(
        m.tags.iter().any(|t| t.name == "Make" || t.name == "Model"),
        "expected at least Make or Model from ExifTool.jpg"
    );
}

#[test]
fn extract_gps_works() {
    let p = images_dir().join("GPS.jpg");
    let m = extract_from_path(&p).expect("extract");
    assert!(
        m.tags.iter().any(|t| t.group == "GPS"),
        "expected GPS tags from GPS.jpg"
    );
}

#[test]
fn extract_png_meta_works() {
    let p = images_dir().join("PNG.png");
    let m = extract_from_path(&p).expect("extract");
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" || t.name == "ImageHeight"),
        "expected PNG dimension tags"
    );
}

#[test]
fn extract_mov_works() {
    let p = images_dir().join("QuickTime.mov");
    let m = extract_from_path(&p).expect("extract");
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "File"),
        "expected File:ImageWidth from QuickTime.mov"
    );
    assert!(
        m.tags.iter().any(|t| t.name == "FileType" && t.value.to_string().contains("MOV")),
        "expected MOV file type"
    );
    assert!(
        m.tags.iter().any(|t| t.name == "Duration" && t.group == "QuickTime"),
        "expected QuickTime Duration"
    );
}

#[test]
fn extract_heic_works() {
    let p = images_dir().join("QuickTime.heic");
    let m = extract_from_path(&p).expect("extract");
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "File"),
        "expected File:ImageWidth from HEIC ispe"
    );
    assert!(
        m.tags.iter().any(|t| t.name == "FileType" && t.value.to_string().contains("HEIC")),
        "expected HEIC file type from ftyp"
    );
}

#[test]
fn strip_jpeg_removes_exif_roundtrip() {
    let p = images_dir().join("ExifTool.jpg");
    let b = fs::read(&p).expect("read");
    let s = strip_metadata(&b, &StripOptions::default()).expect("strip");
    assert!(s.len() < b.len());
    let m = extract(&s).expect("extract stripped");
    assert!(
        !m.tags.iter().any(|t| t.name == "Make"),
        "Make should be removed"
    );
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "File"),
        "still have dimensions"
    );
}

#[test]
fn strip_png_keeps_ihdr() {
    let p = images_dir().join("PNG.png");
    let b = fs::read(&p).expect("read");
    let s = strip_metadata(&b, &StripOptions::default()).expect("strip");
    assert!(s.len() <= b.len());
    let m = extract(&s).expect("extract stripped");
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "PNG"),
        "IHDR dims still present"
    );
}

#[test]
fn strip_mov_keeps_duration_and_video() {
    let p = images_dir().join("QuickTime.mov");
    let b = fs::read(&p).expect("read");
    let s = strip_metadata(&b, &StripOptions::default()).expect("strip");
    assert!(s.len() < b.len());
    let m = extract(&s).expect("extract stripped");
    assert!(
        m.tags.iter().any(|t| t.name == "Duration" && t.group == "QuickTime"),
        "Duration still parseable (stco fix)"
    );
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "File"),
        "video dims still present"
    );
}

#[test]
fn strip_heic_roundtrip() {
    let p = images_dir().join("QuickTime.heic");
    let b = fs::read(&p).expect("read");
    let s = strip_metadata(&b, &StripOptions::default()).expect("strip");
    let m = extract(&s).expect("extract stripped");
    assert!(
        m.tags.iter().any(|t| t.name == "ImageWidth" && t.group == "File"),
        "HEIC dims still readable"
    );
}

#[test]
fn strip_keep_icc_keeps_app2() {
    // Synthetic: SOI + APP1 (Exif) + APP2 (ICC 1/1) + SOS + tail
    let tiff = {
        let v = vec![b'I', b'I', 0x2a, 0, 8, 0, 0, 0, 0];
        v
    };
    let mut j = vec![0xff, 0xd8];
    let pl1 = (2 + 6 + tiff.len()) as u16;
    j.extend_from_slice(&[0xff, 0xe1]);
    j.extend_from_slice(&pl1.to_be_bytes());
    j.extend_from_slice(b"Exif\0\0");
    j.extend_from_slice(&tiff);
    let icc_payload: Vec<u8> = b"ICC_PROFILE\0\x01\x01\x00"
        .iter()
        .copied()
        .chain(std::iter::repeat_n(0u8, 16))
        .collect();
    let pl2 = (2 + icc_payload.len()) as u16;
    j.extend_from_slice(&[0xff, 0xe2]);
    j.extend_from_slice(&pl2.to_be_bytes());
    j.extend_from_slice(&icc_payload);
    j.extend_from_slice(&[0xff, 0xda, 0, 8, 0, 0, 0, 0, 0, 0x3f, 0xff, 0xd9]);
    let def = strip_metadata(&j, &StripOptions::default()).expect("strip");
    assert!(!def.windows(2).any(|w| w == [0xff, 0xe1]));
    let keep = strip_metadata(
        &j,
        &StripOptions {
            keep_icc: true,
            keep_color_info: false,
            keep_jfif: false,
            overwrite_original: false,
        },
    )
    .expect("strip");
    assert!(keep.windows(2).any(|w| w == [0xff, 0xe2]));
}
