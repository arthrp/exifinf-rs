use std::borrow::Cow;
use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::{Error, Result};
use crate::metadata::Metadata;
use crate::tables::lookup_png_text;
use crate::tiff;
use crate::value::Value;
use crate::common::{PNG_SIG};

pub fn parse(meta: &mut Metadata, data: &[u8]) -> Result<()> {
    if data.len() < 8 {
        return Err(Error::BadPng);
    }
    let sig = &data[0..8];
    if sig != PNG_SIG {
        return Err(Error::BadPng);
    }
    let mut p = 8usize;
    while p + 8 <= data.len() {
        let len = u32::from_be_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]) as usize;
        let typ = &data[p + 4..p + 8];
        let dstart = p + 8;
        let dend = dstart.checked_add(len).ok_or(Error::BadPng)?;
        if dend + 4 > data.len() {
            return Err(Error::Truncated);
        }
        let chunk_data = &data[dstart..dend];
        match typ {
            b"IHDR" => parse_ihdr(meta, chunk_data)?,
            b"eXIf" | b"zxIf" => parse_exif_chunk(meta, chunk_data)?,
            b"tEXt" => parse_text(meta, chunk_data)?,
            b"zTXt" => parse_ztxt(meta, chunk_data)?,
            b"iTXt" => parse_itxt(meta, chunk_data)?,
            _ => {}
        }
        p = dend + 4;
        if typ == b"IEND" {
            break;
        }
    }
    Ok(())
}

fn parse_ihdr(meta: &mut Metadata, d: &[u8]) -> Result<()> {
    if d.len() < 13 {
        return Err(Error::BadPng);
    }
    let w = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
    let h = u32::from_be_bytes([d[4], d[5], d[6], d[7]]);
    let bps = d[8];
    let color = d[9];
    meta.push("PNG", "ImageWidth", Value::U32(w));
    meta.push("PNG", "ImageHeight", Value::U32(h));
    meta.push("PNG", "BitsPerSample", Value::U8(bps));
    meta.push("PNG", "ColorType", Value::U8(color));
    Ok(())
}

fn parse_exif_chunk(meta: &mut Metadata, d: &[u8]) -> Result<()> {
    let d = if d.starts_with(b"Exif\0\0") {
        &d[6..]
    } else {
        d
    };
    if d.is_empty() {
        return Ok(());
    }
    let tiff_data: Cow<[u8]> = if d[0] == 0 && d.len() > 5 {
        let mut dec = ZlibDecoder::new(&d[5..]);
        let mut buf = Vec::new();
        dec.read_to_end(&mut buf)
            .map_err(|e| Error::Decompress(e.to_string()))?;
        Cow::Owned(buf)
    } else {
        Cow::Borrowed(d)
    };
    if looks_like_tiff(&tiff_data) {
        tiff::parse_exif_slice(meta, &tiff_data)?;
    }
    Ok(())
}

fn looks_like_tiff(s: &[u8]) -> bool {
    s.len() >= 8
        && ((s[0] == b'I' && s[1] == b'I' && s[2] == 0x2a && s[3] == 0)
            || (s[0] == b'M' && s[1] == b'M' && s[2] == 0 && s[3] == 0x2a))
}

fn parse_text(meta: &mut Metadata, d: &[u8]) -> Result<()> {
    let Some(i) = d.iter().position(|&b| b == 0) else {
        return Ok(());
    };
    let kw = std::str::from_utf8(&d[..i]).unwrap_or("");
    let val = std::str::from_utf8(&d[i + 1..]).unwrap_or("");
    push_png_text(meta, kw, val);
    Ok(())
}

fn parse_ztxt(meta: &mut Metadata, d: &[u8]) -> Result<()> {
    let Some(i) = d.iter().position(|&b| b == 0) else {
        return Ok(());
    };
    let kw = std::str::from_utf8(&d[..i]).unwrap_or("");
    let rest = &d[i + 1..];
    if rest.is_empty() {
        return Ok(());
    }
    let comp = rest[0];
    let raw = &rest[1..];
    let text = if comp == 0 {
        zlib_decompress(raw)?
    } else {
        return Err(Error::Unsupported("zTXt compression"));
    };
    let val = String::from_utf8_lossy(&text);
    push_png_text(meta, kw, &val);
    Ok(())
}

fn parse_itxt(meta: &mut Metadata, d: &[u8]) -> Result<()> {
    let Some(i) = d.iter().position(|&b| b == 0) else {
        return Ok(());
    };
    let kw = std::str::from_utf8(&d[..i]).unwrap_or("");
    let rest = &d[i + 1..];
    if rest.len() < 4 {
        return Ok(());
    }
    let compressed = rest[0];
    let rest = &rest[2..];
    let Some(j) = rest.iter().position(|&b| b == 0) else {
        return Ok(());
    };
    let rest = &rest[j + 1..];
    let Some(k) = rest.iter().position(|&b| b == 0) else {
        return Ok(());
    };
    let val_raw = &rest[k + 1..];
    let val = if compressed == 0 {
        String::from_utf8_lossy(val_raw).into_owned()
    } else {
        String::from_utf8_lossy(&zlib_decompress(val_raw)?).into_owned()
    };
    push_png_text(meta, kw, &val);
    Ok(())
}

fn zlib_decompress(raw: &[u8]) -> Result<Vec<u8>> {
    let mut dec = ZlibDecoder::new(raw);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| Error::Decompress(e.to_string()))?;
    Ok(out)
}

fn push_png_text(meta: &mut Metadata, kw: &str, val: &str) {
    let (group, name): (&str, String) = if let Some(d) = lookup_png_text(kw) {
        (d.group1, d.name.into())
    } else {
        ("PNG", kw.into())
    };
    meta.push(group, name, Value::Utf8(val.into()));
}
