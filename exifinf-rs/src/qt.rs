//! QuickTime / ISO BMFF (MOV, MP4, M4A, HEIC, etc.) read-only metadata.

use crate::byteorder::{read_f32, read_f64, read_i32, read_u32, read_u64, Endian};
use crate::error::{Error, Result};
use crate::iso6709;
use crate::metadata::Metadata;
use crate::qt_tags::{apple_meta_key, classic_udta_atom, ftyp_to_file_mime, ilst_data_atom};
use crate::tiff;
use crate::value::Value;

const XMP_UUID: [u8; 16] = [
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];
const MAC_EPOCH_TO_UNIX: i64 = 2_082_844_800;

struct BmffBox<'a> {
    kind: [u8; 4],
    /// Payload after 8- or 16-byte header. For `uuid`, includes 16 bytes UUID + user data.
    body: &'a [u8],
}

fn read_box(data: &[u8], at: usize) -> Result<Option<(usize, BmffBox<'_>)>> {
    if at + 8 > data.len() {
        return Ok(None);
    }
    let size32 = u32::from_be_bytes(
        *<&[u8; 4]>::try_from(&data[at..at + 4]).map_err(|_| Error::BadQt)?,
    ) as u64;
    let mut kind = [0u8; 4];
    kind.copy_from_slice(&data[at + 4..at + 8]);
    let (header_len, total) = match size32 {
        0 => (8u64, (data.len() - at) as u64),
        1 => {
            if at + 16 > data.len() {
                return Err(Error::Truncated);
            }
            let t = u64::from_be_bytes(
                *<&[u8; 8]>::try_from(&data[at + 8..at + 16]).map_err(|_| Error::BadQt)?,
            );
            (16, t)
        }
        s => (8, s),
    };
    let end = at.checked_add(total as usize).ok_or(Error::BadQt)?;
    if end > data.len() {
        return Err(Error::Truncated);
    }
    if header_len > total {
        return Err(Error::BadQt);
    }
    let body = data
        .get((at + header_len as usize)..end)
        .ok_or(Error::BadQt)?;
    Ok(Some((end, BmffBox { kind, body })))
}

/// Iterate BMFF `Box`es; stops early if a header would extend past `data` (common in truncated samples).
fn for_each_box<F>(data: &[u8], mut f: F) -> Result<()>
where
    F: FnMut([u8; 4], &[u8]) -> Result<()>,
{
    let mut p = 0usize;
    while p < data.len() {
        match read_box(data, p) {
            Ok(Some((end, b))) => {
                f(b.kind, b.body)?;
                p = end;
            }
            Ok(None) => break,
            Err(Error::Truncated) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// `meta` full box: first 4 bytes are version+flags, then child boxes.
fn meta_children(body: &[u8]) -> &[u8] {
    if body.len() < 4 {
        return &[];
    }
    &body[4..]
}

pub fn parse(output: &mut Metadata, data: &[u8]) -> Result<()> {
    let mut major: Option<[u8; 4]> = None;
    let mut minor_version: u32 = 0;
    let mut compat: Vec<[u8; 4]> = Vec::new();
    let mut saw_moov = false;
    let mut p = 0usize;
    while p < data.len() {
        let (end, b) = match read_box(data, p) {
            Ok(Some(x)) => x,
            Ok(None) | Err(Error::Truncated) => break,
            Err(e) => return Err(e),
        };
        match &b.kind {
            b"ftyp" => {
                if b.body.len() >= 4 {
                    let mut m = [0u8; 4];
                    m.copy_from_slice(&b.body[0..4]);
                    major = Some(m);
                }
                if b.body.len() >= 8 {
                    minor_version = u32::from_be_bytes(
                        *<&[u8; 4]>::try_from(&b.body[4..8]).map_err(|_| Error::BadQt)?,
                    );
                }
                let mut cp = 8usize;
                while cp + 4 <= b.body.len() {
                    let mut c = [0u8; 4];
                    c.copy_from_slice(&b.body[cp..cp + 4]);
                    compat.push(c);
                    cp += 4;
                }
            }
            b"moov" => {
                saw_moov = true;
                parse_moov(output, b.body, data)?;
            }
            b"meta" => {
                let inner = meta_children(b.body);
                parse_top_level_meta(output, data, inner)?;
            }
            b"uuid" if b.body.len() >= 16 => {
                if b.body[0..16] == XMP_UUID[0..16]
                    && let Ok(s) = std::str::from_utf8(&b.body[16..]) {
                        output.push("XMP", "XMP", Value::Utf8(s.trim().to_string()));
                    }
            }
            _ => {}
        }
        p = end;
    }

    let (ft, mime) = if let Some(m) = major {
        ftyp_to_file_mime(&m, &compat)
    } else if saw_moov {
        ("MOV".to_string(), "video/quicktime".to_string())
    } else {
        ("HEIF".to_string(), "image/heif".to_string())
    };
    // HEIC with only ftyp+meta+mdat: major is set, so ftyp path applies.
    if major.is_none() && !saw_moov {
        // e.g. fragment — default already HEIF
    }
    output.push("File", "FileType", Value::Utf8(ft));
    output.push("File", "MIMEType", Value::Utf8(mime));
    if let Some(m) = major {
        output.push("File", "MajorBrand", Value::Utf8(fourcc_str(&m)));
        output.push("File", "MinorVersion", Value::U32(minor_version));
    }
    if !compat.is_empty() {
        let s = compat
            .iter()
            .map(fourcc_str)
            .collect::<Vec<_>>()
            .join(", ");
        output.push("File", "CompatibleBrands", Value::Utf8(s));
    }
    Ok(())
}

fn fourcc_str(f: &[u8; 4]) -> String {
    String::from_utf8_lossy(f)
        .trim()
        .trim_end_matches(char::from(0))
        .to_string()
}

/// Walk `moov` box content (not including outer size/type), parse tracks and user `meta`.
fn parse_moov(meta: &mut Metadata, moov_body: &[u8], file: &[u8]) -> Result<()> {
    for_each_box(moov_body, |k, b| {
        if k == *b"mvhd" {
            parse_mvhd(meta, b)?;
        } else if k == *b"trak" {
            parse_trak(meta, b, file)?;
        } else if k == *b"udta" {
            parse_udta(meta, b, file, false)?;
        } else if k == *b"meta" {
            let ch = meta_children(b);
            parse_moov_meta(meta, ch, file)?;
        } else if k == *b"uuid" && b.len() >= 16 && b[0..16] == XMP_UUID[0..16]
            && let Ok(s) = std::str::from_utf8(&b[16..]) {
                meta.push("XMP", "XMP", Value::Utf8(s.trim().to_string()));
            }
        Ok(())
    })
}

fn parse_moov_meta(meta: &mut Metadata, inner: &[u8], _file: &[u8]) -> Result<()> {
    let mut keys: Vec<String> = Vec::new();
    for_each_box(inner, |k, b| {
        if k == *b"keys" {
            keys = parse_keys_box(b);
        }
        Ok(())
    })?;
    for_each_box(inner, |k, b| {
        if k == *b"ilst" {
            if keys.is_empty() {
                parse_ilst_itunes(meta, b)?;
            } else {
                parse_apple_ilst_indexed(meta, b, &keys)?;
            }
        }
        Ok(())
    })
}

fn parse_keys_box(body: &[u8]) -> Vec<String> {
    if body.len() < 8 {
        return Vec::new();
    }
    let n = u32::from_be_bytes([body[4], body[5], body[6], body[7]]) as usize;
    let mut out = Vec::new();
    let mut p = 8usize;
    for _ in 0..n {
        if p >= body.len() {
            break;
        }
        let klen = body[p] as usize;
        p += 1;
        if p + klen > body.len() {
            break;
        }
        let s = String::from_utf8_lossy(&body[p..p + klen]);
        out.push(s.trim_end_matches('\0').to_string());
        p += klen;
    }
    out
}

fn parse_apple_ilst_indexed(meta: &mut Metadata, ilst_body: &[u8], keys: &[String]) -> Result<()> {
    for_each_box(ilst_body, |k, b| {
        let idx = u32::from_be_bytes(k) as usize;
        let key = if idx > 0 && idx <= keys.len() {
            keys[idx - 1].as_str()
        } else {
            return Ok(());
        };
        let Some((g, name)) = apple_meta_key(key) else {
            return Ok(());
        };
        if let Some(v) = extract_ilst_data_value(b) {
            push_gps_or_string(meta, g, name, &v);
        }
        Ok(())
    })
}

fn parse_ilst_itunes(meta: &mut Metadata, ilst_body: &[u8]) -> Result<()> {
    for_each_box(ilst_body, |k, b| {
        let four = k;
        if let Some((g, name)) = ilst_data_atom(&four)
            && let Some(v) = extract_ilst_data_value(b) {
                push_gps_or_string(meta, g, name, &v);
            }
        Ok(())
    })
}

fn extract_ilst_data_value(b: &[u8]) -> Option<String> {
    let mut p = 0usize;
    while p < b.len() {
        let next = read_box(b, p);
        let (end, bx) = match next {
            Ok(Some(x)) => x,
            Ok(None) | Err(_) => break,
        };
        if bx.kind == *b"data" && bx.body.len() > 8 {
            let typ = bx.body[8];
            let payload = &bx.body[12..];
            let s = match typ {
                1..=3 => String::from_utf8_lossy(payload).into_owned(),
                21 if payload.len() >= 4 => read_i32(payload, 0, Endian::Big)
                    .map(|i| i.to_string())
                    .unwrap_or_default(),
                23 if payload.len() >= 4 => read_f32(payload, 0, Endian::Big)
                    .map(|f| f.to_string())
                    .unwrap_or_default(),
                24 if payload.len() >= 8 => read_f64(payload, 0, Endian::Big)
                    .map(|f| f.to_string())
                    .unwrap_or_default(),
                _ => String::from_utf8_lossy(payload).into_owned(),
            };
            return Some(s.trim().to_string());
        }
        p = end;
    }
    None
}

fn push_gps_or_string(meta: &mut Metadata, g: &str, name: &str, v: &str) {
    if name == "GPSCoordinates"
        && let Some((la, lo, alt)) = iso6709::parse_iso6709(v) {
            meta.push(
                "QuickTime",
                "GPSCoordinates",
                Value::Utf8(format!("{la:.6}, {lo:.6}")),
            );
            meta.push("QuickTime", "GPSLatitude", Value::F64(la));
            meta.push("QuickTime", "GPSLongitude", Value::F64(lo));
            if let Some(a) = alt {
                meta.push("QuickTime", "GPSAltitude", Value::F64(a));
            }
            return;
        }
    meta.push(g, name, Value::Utf8(v.to_string()));
}

fn parse_top_level_meta(meta: &mut Metadata, file: &[u8], inner: &[u8]) -> Result<()> {
    walk_ispe_for_dims(meta, inner)?;
    heic_embedded_tiff_exif(meta, file)?;
    Ok(())
}

/// Find `ispe` (HEIF spatial extent) anywhere under a `meta` tree.
fn walk_ispe_for_dims(meta: &mut Metadata, buf: &[u8]) -> Result<()> {
    for_each_box(buf, |k, b| {
        if k == *b"ispe" {
            if b.len() >= 12 {
                let w = read_u32(b, 4, Endian::Big)?;
                let h = read_u32(b, 8, Endian::Big)?;
                if w > 0 && h > 0 {
                    meta.push("File", "ImageWidth", Value::U32(w));
                    meta.push("File", "ImageHeight", Value::U32(h));
                }
            }
        } else if k == *b"iprp" || k == *b"ipco" {
            walk_ispe_for_dims(meta, b)?;
        }
        Ok(())
    })
}

/// Scan for a TIFF/EXIF payload (some HEIC files embed EXIF in `mdat` without a matching iloc in tests).
fn heic_embedded_tiff_exif(meta: &mut Metadata, file: &[u8]) -> Result<()> {
    if file.len() < 8 {
        return Ok(());
    }
    for i in 0..=file.len().saturating_sub(8) {
        if (file[i] == b'I' && file[i + 1] == b'I' && file[i + 2] == 0x2a && file[i + 3] == 0
            || file[i] == b'M' && file[i + 1] == b'M' && file[i + 2] == 0 && file[i + 3] == 0x2a)
            && tiff::parse_exif_slice(meta, &file[i..]).is_ok() {
                break;
            }
    }
    Ok(())
}

fn parse_udta(meta: &mut Metadata, body: &[u8], _file: &[u8], _nested: bool) -> Result<()> {
    for_each_box(body, |k, b| {
        if k == *b"meta" {
            let ch = meta_children(b);
            let mut keys: Vec<String> = Vec::new();
            for_each_box(ch, |k2, b2| {
                if k2 == *b"keys" {
                    keys = parse_keys_box(b2);
                }
                Ok(())
            })?;
            for_each_box(ch, |k2, b2| {
                if k2 == *b"ilst" {
                    if keys.is_empty() {
                        parse_ilst_itunes(meta, b2)?;
                    } else {
                        parse_apple_ilst_indexed(meta, b2, &keys)?;
                    }
                }
                Ok(())
            })?;
        } else if let Some((g, name)) = classic_udta_atom(&k) {
            if name == "GPSCoordinates" {
                if let Ok(s) = udta_string_lossy(b) {
                    push_gps_or_string(meta, g, name, &s);
                }
            } else {
                if let Ok(s) = udta_string_lossy(b) {
                    meta.push(g, name, Value::Utf8(s));
                }
            }
        }
        Ok(())
    })
}

fn udta_string_lossy(b: &[u8]) -> Result<String> {
    if b.len() < 2 {
        return Ok(String::from_utf8_lossy(b).trim().to_string());
    }
    // Common: 2-byte lang, then UTF-8; or 16-bit length then string
    let s = if b.len() > 4 && b[0] == 0 && b[1] == 0 {
        String::from_utf8_lossy(&b[4..]).into_owned()
    } else {
        String::from_utf8_lossy(&b[2..]).into_owned()
    };
    Ok(s.trim().trim_matches(char::from(0)).to_string())
}

fn parse_mvhd(meta: &mut Metadata, b: &[u8]) -> Result<()> {
    if b.len() < 4 {
        return Ok(());
    }
    let ver = b[0];
    let (ts, dur, c, m) = if ver == 1 && b.len() >= 32 {
        let c = read_u64(b, 4, Endian::Big)?;
        let m = read_u64(b, 12, Endian::Big)?;
        let ts = read_u32(b, 20, Endian::Big)?;
        let dur = read_u64(b, 24, Endian::Big)?;
        (ts, dur, c, m)
    } else if b.len() >= 20 {
        let c = u64::from(read_u32(b, 4, Endian::Big)?);
        let m = u64::from(read_u32(b, 8, Endian::Big)?);
        let ts = read_u32(b, 12, Endian::Big)?;
        let dur = u64::from(read_u32(b, 16, Endian::Big)?);
        (ts, dur, c, m)
    } else {
        return Ok(());
    };
    meta.push("QuickTime", "TimeScale", Value::U32(ts));
    if ts > 0 {
        let secs = dur as f64 / f64::from(ts);
        meta.push("QuickTime", "Duration", Value::F64(secs));
    }
    if let Some(s) = mac_time_to_exif_string(c) {
        meta.push("QuickTime", "CreateDate", Value::Utf8(s));
    }
    if let Some(s) = mac_time_to_exif_string(m) {
        meta.push("QuickTime", "ModifyDate", Value::Utf8(s));
    }
    Ok(())
}

fn mac_time_to_exif_string(t: u64) -> Option<String> {
    if t == 0 {
        return None;
    }
    let unix = (t as i64).saturating_sub(MAC_EPOCH_TO_UNIX);
    if unix < 0 {
        return None;
    }
    Some(format_unix_utc(unix as u64))
}

/// Format `unix` seconds as `YYYY:MM:DD HH:MM:SS` (UTC).
fn format_unix_utc(unix: u64) -> String {
    const SECS_PER_DAY: u64 = 86400;
    let secs = unix % SECS_PER_DAY;
    let mut rem = unix / SECS_PER_DAY;
    let mut y: i64 = 1970;
    while rem >= year_len(y) {
        rem -= year_len(y);
        y += 1;
    }
    let doy = rem;
    let mut d = doy;
    let mut m = 1u32;
    for month in 1u32..=12 {
        let dim = days_in_month(y, month);
        if d < dim {
            m = month;
            break;
        }
        d -= dim;
    }
    let day = d + 1;
    let hh = (secs / 3600) as u32;
    let mm = ((secs % 3600) / 60) as u32;
    let ss = (secs % 60) as u32;
    format!("{y:04}:{m:02}:{day:02} {hh:02}:{mm:02}:{ss:02}")
}

fn year_len(y: i64) -> u64 {
    if is_leap_year(y) {
        366
    } else {
        365
    }
}

fn days_in_month(y: i64, m: u32) -> u64 {
    match m {
        2 if is_leap_year(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn parse_tkhd_dims(b: &[u8]) -> Option<(u32, u32)> {
    if b.is_empty() {
        return None;
    }
    // Version 0 only (v1 display dimensions at different offsets; skip if unknown)
    if b[0] != 0 {
        return None;
    }
    let woff = 76usize;
    if b.len() < woff + 8 {
        return None;
    }
    let w = u32::from_be_bytes(b[woff..woff + 4].try_into().ok()?) as f64 / 65536.0;
    let h = u32::from_be_bytes(b[woff + 4..woff + 8].try_into().ok()?) as f64 / 65536.0;
    if w < 1.0 || h < 1.0 {
        return None;
    }
    Some((w as u32, h as u32))
}

fn parse_trak(meta: &mut Metadata, trak: &[u8], _file: &[u8]) -> Result<()> {
    let mut kind: Option<[u8; 4]> = None;
    let mut tk_dim: Option<(u32, u32)> = None;
    let mut md_scale: u32 = 0;
    let mut stts_sum: u64 = 0;
    let mut stts_cnt: u64 = 0;
    let mut video_codec: Option<String> = None;
    for_each_box(trak, |k, b| {
        if k == *b"tkhd" {
            tk_dim = parse_tkhd_dims(b);
        } else if k == *b"mdia" {
            for_each_box(b, |k2, b2| {
                if k2 == *b"mdhd" {
                    if b2.len() >= 4 {
                        let v = b2[0];
                        if v == 1 && b2.len() >= 32 {
                            md_scale = read_u32(b2, 20, Endian::Big)?;
                        } else if b2.len() >= 20 {
                            md_scale = read_u32(b2, 12, Endian::Big)?;
                        }
                    }
                } else if k2 == *b"hdlr" {
                    // ISO + QuickTime: pre_defined, `mhlr`/`dhlr` …, then `vide` / `soun` at 12..16
                    if b2.len() >= 16 {
                        let mut s = [0u8; 4];
                        s.copy_from_slice(&b2[12..16]);
                        kind = Some(s);
                    }
                } else if k2 == *b"minf" {
                    for_each_box(b2, |k3, b3| {
                        if k3 == *b"stbl" {
                            for_each_box(b3, |k4, b4| {
                                if k4 == *b"stts" {
                                    stts_parse_summary(b4, &mut stts_cnt, &mut stts_sum)?;
                                } else if k4 == *b"stsd" && b4.len() >= 16
                                    && let Some(cc) = stsd_first_codec(b4) {
                                        video_codec = Some(cc);
                                    }
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })?;
                }
                Ok(())
            })?;
        }
        Ok(())
    })?;
    // Video: explicit `vide` handler, or a non-sound track with a display `tkhd` (handler may be missed if
    // a nested `for_each_box` stops early on truncated sub-boxes).
    let is_audio = kind == Some(*b"soun");
    let is_video = kind == Some(*b"vide");
    let is_videoish = is_video
        || (tk_dim.is_some() && !is_audio);
    if is_videoish {
        if let Some((w, h)) = tk_dim {
            meta.push("File", "ImageWidth", Value::U32(w));
            meta.push("File", "ImageHeight", Value::U32(h));
        }
        if md_scale > 0 && stts_cnt > 0 && stts_sum > 0 {
            let fps = (stts_cnt as f64) * f64::from(md_scale) / (stts_sum as f64);
            meta.push("Track1", "VideoFrameRate", Value::F64(fps));
        }
        if let Some(c) = video_codec {
            meta.push("Track1", "VideoCodec", Value::Utf8(c));
        }
    }
    if is_audio
        && md_scale > 0 {
            meta.push("Track0", "AudioFormat", Value::Utf8("soun".to_string()));
            meta.push("Track0", "AudioSampleRate", Value::F64(f64::from(md_scale)));
        }
    Ok(())
}

fn stsd_first_codec(b: &[u8]) -> Option<String> {
    if b.len() < 16 {
        return None;
    }
    let c = u32::from_be_bytes(b[4..8].try_into().ok()?) as usize;
    if c < 1 {
        return None;
    }
    let t = b.get(12..16)?;
    Some(String::from_utf8_lossy(t).to_string())
}

fn stts_parse_summary(b: &[u8], cnt: &mut u64, sum: &mut u64) -> Result<()> {
    if b.len() < 8 {
        return Ok(());
    }
    let n = read_u32(b, 4, Endian::Big)? as usize;
    let mut p = 8usize;
    for _ in 0..n {
        if p + 8 > b.len() {
            break;
        }
        let c = u64::from(read_u32(b, p, Endian::Big)?);
        let d = u64::from(read_u32(b, p + 4, Endian::Big)?);
        p += 8;
        *sum = sum.saturating_add(c * d);
        *cnt = cnt.saturating_add(c);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mdat` with 64-bit size header (size = 1, extended size = 16 + 8 payload).
    #[test]
    fn read_box_64bit_size() {
        let mut v = vec![0x00, 0x00, 0x00, 0x01];
        v.extend_from_slice(b"mdat");
        v.extend_from_slice(&24u64.to_be_bytes());
        v.extend_from_slice(b"12345678");
        let r = read_box(&v, 0).expect("read");
        let (end, b) = r.unwrap();
        assert_eq!(end, v.len());
        assert_eq!(b.kind, *b"mdat");
        assert_eq!(b.body, b"12345678");
    }

    #[test]
    fn minimal_ftyp_moov_mvhd_yields_duration() {
        let mut m = Metadata::default();
        let mut buf: Vec<u8> = vec![];
        // 8 (size+type) + 12 (major + minor + one compatible brand) = 20. Size must
        // match bytes written, or the `moov` would overlap the declared `ftyp` span.
        buf.extend_from_slice(&20u32.to_be_bytes());
        buf.extend_from_slice(b"ftyp");
        buf.extend_from_slice(b"isom");
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(b"mp41");
        let moov_body_len = 108u32;
        buf.extend_from_slice(&(moov_body_len + 8).to_be_bytes());
        buf.extend_from_slice(b"moov");
        buf.extend_from_slice(&moov_body_len.to_be_bytes());
        buf.extend_from_slice(b"mvhd");
        let mut mvhd = vec![0u8; 100];
        mvhd[0] = 0;
        mvhd[12..16].copy_from_slice(&600u32.to_be_bytes());
        mvhd[16..20].copy_from_slice(&3000u32.to_be_bytes());
        buf.extend_from_slice(&mvhd);
        parse(&mut m, &buf).expect("parse");
        assert!(m.tags.iter().any(|t| t.name == "Duration" && t.group == "QuickTime"));
        assert!(m.tags.iter().any(|t| t.name == "FileType" && t.group == "File"));
    }
}
