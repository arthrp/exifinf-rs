use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TagJson {
    name: String,
    writable: Option<String>,
    group1: String,
    print_conv: Option<BTreeMap<String, String>>,
    sub_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PngRoot {
    chunks: BTreeMap<String, TagJson>,
    textual: BTreeMap<String, TagJson>,
}

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("MANIFEST_DIR");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("tables.rs");
    let data = Path::new(&manifest).join("data");

    let exif: BTreeMap<String, TagJson> =
        serde_json::from_str(&fs::read_to_string(data.join("exif.json")).expect("exif.json"))
            .expect("parse exif.json");
    let gps: BTreeMap<String, TagJson> =
        serde_json::from_str(&fs::read_to_string(data.join("gps.json")).expect("gps.json"))
            .expect("parse gps.json");
    let png: PngRoot =
        serde_json::from_str(&fs::read_to_string(data.join("png.json")).expect("png.json"))
            .expect("parse png.json");

    let mut buf = String::new();
    buf.push_str("use crate::format::Format;\n");
    buf.push_str("use crate::tag_def::{PngChunkDef, PngTextDef, PrintConv, SubDir, TagDef};\n\n");

    emit_table(&mut buf, "EXIF_MAIN", &exif);
    emit_table(&mut buf, "GPS_MAIN", &gps);
    emit_png(&mut buf, &png);

    fs::write(&out, buf).expect("write tables.rs");
    println!("cargo:rerun-if-changed=data/exif.json");
    println!("cargo:rerun-if-changed=data/gps.json");
    println!("cargo:rerun-if-changed=data/png.json");
    println!("cargo:rerun-if-changed=build.rs");
}

fn emit_table(buf: &mut String, name: &str, map: &BTreeMap<String, TagJson>) {
    buf.push_str(&format!("pub static {name}: &[(u16, TagDef)] = &[\n"));
    for (k, v) in map {
        let id = parse_hex_key(k);
        let def = emit_tag_def(v);
        buf.push_str(&format!("    (0x{:04x}, {def}),\n", id));
    }
    buf.push_str("];\n\n");
}

fn emit_png(buf: &mut String, png: &PngRoot) {
    buf.push_str("pub static PNG_CHUNKS: &[([u8; 4], PngChunkDef)] = &[\n");
    for (chunk, v) in &png.chunks {
        if chunk.len() != 4 {
            continue;
        }
        let b: Vec<u8> = chunk.bytes().collect();
        let def = emit_png_chunk_def(v);
        buf.push_str(&format!(
            "    ([{}, {}, {}, {}], {def}),\n",
            b[0], b[1], b[2], b[3]
        ));
    }
    buf.push_str("];\n\n");

    buf.push_str("pub static PNG_TEXTUAL: &[(&'static str, PngTextDef)] = &[\n");
    for (kw, v) in &png.textual {
        let def = emit_png_text_def(v);
        let kw_lit = format!("{:?}", kw);
        buf.push_str(&format!("    ({kw_lit}, {def}),\n"));
    }
    buf.push_str("];\n\n");
}

fn emit_tag_def(v: &TagJson) -> String {
    let name = format!("{:?}", v.name);
    let fmt = v
        .writable
        .as_deref()
        .and_then(format_from_writable)
        .map(|f| format!("Some(Format::{f})"))
        .unwrap_or_else(|| "None".to_string());
    let pc = emit_print_conv(v.print_conv.as_ref());
    let sd = match v.sub_directory.as_deref() {
        Some("ExifIFD") => "SubDir::ExifIfd",
        Some("GPS") => "SubDir::GpsIfd",
        Some("InteropIFD") => "SubDir::InteropIfd",
        Some("SubIFD") => "SubDir::SubIfd",
        Some("MakerNotes") => "SubDir::MakerNotes",
        _ => "SubDir::None",
    };
    let g1 = format!("{:?}", v.group1);
    format!("TagDef {{ name: {name}, format: {fmt}, print_conv: {pc}, sub_dir: {sd}, group1: {g1} }}")
}

fn emit_png_chunk_def(v: &TagJson) -> String {
    let name = format!("{:?}", v.name);
    let g1 = format!("{:?}", v.group1);
    let pc = emit_print_conv(v.print_conv.as_ref());
    format!("PngChunkDef {{ chunk_name: {name}, group1: {g1}, print_conv: {pc} }}")
}

fn emit_png_text_def(v: &TagJson) -> String {
    let name = format!("{:?}", v.name);
    let g1 = format!("{:?}", v.group1);
    let pc = emit_print_conv(v.print_conv.as_ref());
    format!("PngTextDef {{ name: {name}, group1: {g1}, print_conv: {pc} }}")
}

fn emit_print_conv(pc: Option<&BTreeMap<String, String>>) -> String {
    let Some(m) = pc else {
        return "PrintConv::None".to_string();
    };
    if m.is_empty() {
        return "PrintConv::None".to_string();
    }
    let mut all_int = true;
    let mut pairs: Vec<(i64, String)> = Vec::new();
    let mut str_pairs: Vec<(String, String)> = Vec::new();
    for (k, v) in m {
        if let Ok(i) = k.parse::<i64>() {
            pairs.push((i, v.clone()));
        } else {
            all_int = false;
            break;
        }
    }
    if all_int && !pairs.is_empty() {
        pairs.sort_by_key(|p| p.0);
        let inner: String = pairs
            .iter()
            .map(|(i, s)| format!("({}, {:?})", i, s))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("PrintConv::IntMap(&[{inner}])");
    }
    for (k, v) in m {
        str_pairs.push((k.clone(), v.clone()));
    }
    str_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let inner: String = str_pairs
        .iter()
        .map(|(k, v)| format!("({:?}, {:?})", k, v))
        .collect::<Vec<_>>()
        .join(", ");
    format!("PrintConv::StrMap(&[{inner}])")
}

fn parse_hex_key(k: &str) -> u16 {
    let s = k.strip_prefix("0x").unwrap_or(k);
    u16::from_str_radix(s, 16).unwrap_or_else(|_| panic!("bad key {k:?}"))
}

fn format_from_writable(w: &str) -> Option<&'static str> {
    Some(match w {
        "int8u" => "Int8u",
        "string" => "String",
        "int16u" => "Int16u",
        "int32u" => "Int32u",
        "rational64u" => "Rational64u",
        "int8s" => "Int8s",
        "undef" | "binary" => "Undef",
        "int16s" => "Int16s",
        "int32s" => "Int32s",
        "rational64s" => "Rational64s",
        "float" => "Float",
        "double" => "Double",
        "ifd" => "Ifd",
        "unicode" => "Unicode",
        "complex" => "Complex",
        "int64u" => "Int64u",
        "int64s" => "Int64s",
        "ifd64" => "Ifd64",
        "utf8" => "Utf8",
        _ => return None,
    })
}
