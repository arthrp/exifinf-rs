use crate::error::{Error, Result};
use crate::metadata::Metadata;
use crate::tiff;

const EXIF_HDR: &[u8] = b"Exif\0\0";

pub fn parse(meta: &mut Metadata, data: &[u8]) -> Result<()> {
    if data.len() < 4 || data[0..2] != [0xff, 0xd8] {
        return Err(Error::BadJpeg);
    }
    let segs = collect_segments_until_sos(data)?;
    let mut i = 0;
    while i < segs.len() {
        let (marker, payload) = &segs[i];
        if *marker == 0xe1 && exif_header_len(payload).is_some() {
            let mut buf = strip_exif_payload(payload)?;
            let mut j = i;
            while j + 1 < segs.len()
                && segs[j + 1].0 == 0xe1
                && is_exif_continuation(&segs[j + 1].1)
            {
                j += 1;
                buf.extend_from_slice(&strip_exif_payload(&segs[j].1)?);
            }
            i = j;
            tiff::parse_exif_slice(meta, &buf)?;
        } else if is_sof(*marker)
            && let Some((w, h, bps, comps)) = parse_sof(payload) {
                meta.push("File", "ImageWidth", crate::value::Value::U32(w));
                meta.push("File", "ImageHeight", crate::value::Value::U32(h));
                meta.push("File", "BitsPerSample", crate::value::Value::U8(bps));
                meta.push(
                    "File",
                    "ColorComponents",
                    crate::value::Value::U8(comps),
                );
            }
        i += 1;
    }
    Ok(())
}

fn collect_segments_until_sos(data: &[u8]) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut p = 2usize;
    let mut out = Vec::new();
    while p < data.len() {
        if data[p] != 0xff {
            return Err(Error::BadJpeg);
        }
        while p < data.len() && data[p] == 0xff {
            p += 1;
        }
        if p >= data.len() {
            break;
        }
        let m = data[p];
        p += 1;
        if m == 0xd8 || m == 0xd9 || (0xd0..=0xd7).contains(&m) || m == 0x01 {
            continue;
        }
        if m == 0xda {
            break; // Start Of Scan
        }
        if p + 2 > data.len() {
            return Err(Error::Truncated);
        }
        let seglen = u16::from_be_bytes([data[p], data[p + 1]]) as usize;
        p += 2;
        if seglen < 2 || p + seglen - 2 > data.len() {
            return Err(Error::BadJpeg);
        }
        let payload = data[p..p + seglen - 2].to_vec();
        p += seglen - 2;
        out.push((m, payload));
    }
    Ok(out)
}

fn is_sof(m: u8) -> bool {
    (m & 0xf0) == 0xc0 && (m == 0xc0 || (m & 0x03) != 0) && ![0xc4, 0xc8, 0xcc].contains(&m)
}

fn parse_sof(payload: &[u8]) -> Option<(u32, u32, u8, u8)> {
    if payload.len() < 6 {
        return None;
    }
    let bps = payload[0];
    let h = u32::from(u16::from_be_bytes([payload[1], payload[2]]));
    let w = u32::from(u16::from_be_bytes([payload[3], payload[4]]));
    let comps = payload[5];
    Some((w, h, bps, comps))
}

fn exif_header_len(payload: &[u8]) -> Option<usize> {
    let p = payload
        .windows(EXIF_HDR.len())
        .position(|w| w == EXIF_HDR)?;
    Some(p + EXIF_HDR.len())
}

fn strip_exif_payload(payload: &[u8]) -> Result<Vec<u8>> {
    let h = exif_header_len(payload).ok_or(Error::BadJpeg)?;
    Ok(payload[h..].to_vec())
}

fn is_exif_continuation(payload: &[u8]) -> bool {
    if !payload.starts_with(EXIF_HDR) {
        return false;
    }
    let after = &payload[EXIF_HDR.len()..];
    !looks_like_tiff(after)
}

fn looks_like_tiff(s: &[u8]) -> bool {
    s.len() >= 8
        && ((s[0] == b'I' && s[1] == b'I' && s[2] == 0x2a && s[3] == 0)
            || (s[0] == b'M' && s[1] == b'M' && s[2] == 0 && s[3] == 0x2a))
}
