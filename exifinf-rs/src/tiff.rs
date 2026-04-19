use std::collections::HashSet;

use crate::byteorder::{read_f32, read_f64, read_i16, read_i32, read_i64, read_u16, read_u32, read_u64, Endian};
use crate::error::{Error, Result};
use crate::format::Format;
use crate::metadata::Metadata;
use crate::tag_def::SubDir;
use crate::tables::{lookup_exif, lookup_gps};
use crate::value::{Rational, SRational, Value};

pub fn parse_exif_slice(meta: &mut Metadata, tiff: &[u8]) -> Result<()> {
    if tiff.len() < 8 {
        return Err(Error::BadTiff);
    }
    let endian = match &tiff[0..2] {
        b"II" => Endian::Little,
        b"MM" => Endian::Big,
        _ => return Err(Error::BadTiff),
    };
    let magic = read_u16(tiff, 2, endian)?;
    if magic != 42 {
        return Err(Error::BadTiff);
    }
    let ifd0 = read_u32(tiff, 4, endian)? as usize;
    let mut visited = HashSet::new();
    let mut off = ifd0;
    let mut idx = 0u32;
    loop {
        let label = if idx == 0 {
            "IFD0".to_string()
        } else if idx == 1 {
            "IFD1".to_string()
        } else {
            format!("IFD{}", idx)
        };
        let next = walk_ifd(meta, tiff, endian, off, &label, false, &mut visited)?;
        if next == 0 {
            break;
        }
        off = next as usize;
        idx += 1;
    }
    Ok(())
}

fn walk_ifd(
    meta: &mut Metadata,
    buf: &[u8],
    e: Endian,
    ifd_off: usize,
    group: &str,
    in_gps: bool,
    visited: &mut HashSet<u32>,
) -> Result<u32> {
    if ifd_off + 2 > buf.len() {
        return Err(Error::BadTiff);
    }
    let key = ifd_off as u32;
    if !visited.insert(key) {
        return Ok(0);
    }
    let n = read_u16(buf, ifd_off, e)? as usize;
    let mut pos = ifd_off + 2;
    for _ in 0..n {
        if pos + 12 > buf.len() {
            return Err(Error::BadTiff);
        }
        let tag = read_u16(buf, pos, e)?;
        let typ = read_u16(buf, pos + 2, e)?;
        let count = read_u32(buf, pos + 4, e)? as usize;
        let value_field_off = pos + 8;
        pos += 12;

        let fmt = Format::from_u16(typ).ok_or(Error::UnknownFormat)?;
        let comp = fmt.size().checked_mul(count).ok_or(Error::BadTiff)?;
        let data: &[u8] = if comp <= 4 {
            buf.get(value_field_off..value_field_off + comp)
                .ok_or(Error::BadTiff)?
        } else {
            let ext = read_u32(buf, value_field_off, e)? as usize;
            let end = ext.checked_add(comp).ok_or(Error::BadTiff)?;
            if end > buf.len() {
                return Err(Error::BadTiff);
            }
            &buf[ext..end]
        };

        let def = if in_gps {
            lookup_gps(tag)
        } else {
            lookup_exif(tag)
        };
        let sub = def.map(|d| d.sub_dir).unwrap_or(SubDir::None);

        if tag == 0x927c || sub == SubDir::MakerNotes {
            meta.push_id(group, "MakerNote", tag, Value::Undef(data.to_vec()));
            continue;
        }

        match sub {
            SubDir::ExifIfd | SubDir::GpsIfd | SubDir::InteropIfd => {
                let child_off = read_u32(data, 0, e)? as usize;
                if let Some(d) = def {
                    let pv = read_value_from_slice(data, e, Format::Int32u, 1)?;
                    meta.push_id(group, d.name, tag, pv);
                }
                let child_gps = matches!(sub, SubDir::GpsIfd);
                let child_group = match sub {
                    SubDir::ExifIfd => "ExifIFD",
                    SubDir::GpsIfd => "GPS",
                    SubDir::InteropIfd => "InteropIFD",
                    _ => group,
                };
                walk_ifd(meta, buf, e, child_off, child_group, child_gps, visited)?;
            }
            SubDir::SubIfd => {
                if let Some(d) = def {
                    let pv = read_value_from_slice(data, e, fmt, count)?;
                    meta.push_id(group, d.name, tag, pv);
                }
                if fmt == Format::Int32u || fmt == Format::Ifd {
                    for i in 0..count {
                        let o = read_u32(data, i * 4, e)? as usize;
                        walk_ifd(meta, buf, e, o, "SubIFD", in_gps, visited)?;
                    }
                }
            }
            SubDir::None | SubDir::MakerNotes => {
                let v = read_value_from_slice(data, e, fmt, count)?;
                let name = def
                    .map(|d| d.name.to_string())
                    .unwrap_or_else(|| format!("Tag 0x{tag:04x}"));
                meta.push_id(group, name, tag, v);
            }
        }
    }

    if pos + 4 > buf.len() {
        return Ok(0);
    }
    read_u32(buf, pos, e)
}

fn read_value_from_slice(data: &[u8], e: Endian, fmt: Format, count: usize) -> Result<Value> {
    let sz = fmt.size().checked_mul(count).ok_or(Error::BadTiff)?;
    if data.len() < sz {
        return Err(Error::Truncated);
    }
    Ok(match fmt {
        Format::Int8u => {
            if count == 1 {
                Value::U8(data[0])
            } else {
                Value::Undef(data[..sz].to_vec())
            }
        }
        Format::String | Format::Utf8 => {
            let mut s = String::from_utf8_lossy(&data[..sz]).into_owned();
            if let Some(z) = s.find('\0') {
                s.truncate(z);
            }
            if fmt == Format::Utf8 {
                Value::Utf8(s)
            } else {
                Value::Ascii(s)
            }
        }
        Format::Int16u => {
            if count == 1 {
                Value::U16(read_u16(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_u16(data, i * 2, e)?);
                }
                Value::U16s(v)
            }
        }
        Format::Int32u | Format::Ifd => {
            if count == 1 {
                Value::U32(read_u32(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_u32(data, i * 4, e)?);
                }
                Value::U32s(v)
            }
        }
        Format::Int8s => {
            if count == 1 {
                Value::I8(data[0] as i8)
            } else {
                Value::I8s(data[..count].iter().map(|&b| b as i8).collect())
            }
        }
        Format::Int16s => {
            if count == 1 {
                Value::I16(read_i16(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_i16(data, i * 2, e)?);
                }
                Value::I16s(v)
            }
        }
        Format::Int32s => {
            if count == 1 {
                Value::I32(read_i32(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_i32(data, i * 4, e)?);
                }
                Value::I32s(v)
            }
        }
        Format::Rational64u => {
            let mut rats = Vec::with_capacity(count);
            for i in 0..count {
                let o = i * 8;
                rats.push(Rational {
                    num: read_u32(data, o, e)?,
                    den: read_u32(data, o + 4, e)?,
                });
            }
            if count == 1 {
                Value::Rational(rats[0].clone())
            } else {
                Value::Rationals(rats)
            }
        }
        Format::Rational64s => {
            if count == 1 {
                Value::SRational(SRational {
                    num: read_i32(data, 0, e)?,
                    den: read_i32(data, 4, e)?,
                })
            } else {
                Value::Undef(data[..sz].to_vec())
            }
        }
        Format::Float => {
            if count == 1 {
                Value::F32(read_f32(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_f32(data, i * 4, e)?);
                }
                Value::F32s(v)
            }
        }
        Format::Double => {
            if count == 1 {
                Value::F64(read_f64(data, 0, e)?)
            } else {
                let mut v = Vec::with_capacity(count);
                for i in 0..count {
                    v.push(read_f64(data, i * 8, e)?);
                }
                Value::F64s(v)
            }
        }
        Format::Undef | Format::Unicode | Format::Complex => Value::Undef(data[..sz].to_vec()),
        Format::Int64u => {
            if count == 1 {
                Value::U64(read_u64(data, 0, e)?)
            } else {
                Value::Undef(data[..sz].to_vec())
            }
        }
        Format::Int64s => {
            if count == 1 {
                Value::I64(read_i64(data, 0, e)?)
            } else {
                Value::Undef(data[..sz].to_vec())
            }
        }
        Format::Ifd64 => Value::U64(read_u64(data, 0, e)?),
    })
}
