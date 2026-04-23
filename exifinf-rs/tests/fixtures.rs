//! Integration checks against sample images in the workspace `img/` directory.

use std::path::PathBuf;

use exifinf_rs::extract_from_path;

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
