use crate::error::{Error, Result};

/// Strip metadata segments from a JPEG, matching a conservative subset of
/// `exiftool -all=` (APP/COM; optional JFIF/ICC per options).
pub fn strip(data: &[u8], opts: &crate::StripOptions) -> Result<Vec<u8>> {
    if data.len() < 2 || &data[0..2] != [0xff, 0xd8] {
        return Err(Error::BadJpeg);
    }
    let mut out = vec![0xff, 0xd8];
    let mut p = 2usize;
    while p < data.len() {
        if data[p] != 0xff {
            return Err(Error::BadJpeg);
        }
        let seg_start = p;
        p += 1;
        while p < data.len() && data[p] == 0xff {
            p += 1;
        }
        if p >= data.len() {
            break;
        }
        let m = data[p];
        p += 1;
        if m == 0xd8
            || m == 0xd9
            || (0xd0..=0xd7).contains(&m)
            || m == 0x01
        {
            continue;
        }
        if p + 2 > data.len() {
            return Err(Error::Truncated);
        }
        let seglen = u16::from_be_bytes([data[p], data[p + 1]]) as usize;
        p += 2;
        if seglen < 2 {
            return Err(Error::BadJpeg);
        }
        let end = seg_start
            .checked_add(2)
            .and_then(|a| a.checked_add(seglen))
            .ok_or(Error::BadJpeg)?;
        if end > data.len() {
            return Err(Error::BadJpeg);
        }
        if m == 0xda {
            // Start Of Scan: always keep segment, then all entropy + RST + EOI
            out.extend_from_slice(&data[seg_start..end]);
            out.extend_from_slice(&data[end..]);
            return Ok(out);
        }
        let payload = &data[p..end];
        p = end;
        if keep_marked_segment(m, payload, opts) {
            out.extend_from_slice(&data[seg_start..end]);
        }
    }
    Err(Error::BadJpeg)
}

fn is_sof(m: u8) -> bool {
    (m & 0xf0) == 0xc0 && (m == 0xc0 || (m & 0x03) != 0) && ![0xc4, 0xc8, 0xcc].contains(&m)
}

fn is_icc_app2_payload(payload: &[u8]) -> bool {
    payload.len() > 1 && payload.starts_with(b"ICC_PROFILE\0")
}

fn is_jfif_app0(payload: &[u8]) -> bool {
    payload.starts_with(b"JFIF\0")
}

fn keep_marked_segment(marker: u8, payload: &[u8], opts: &crate::StripOptions) -> bool {
    if marker == 0xfe {
        return false; // COM
    }
    if (0xe0..=0xef).contains(&marker) {
        if opts.keep_jfif && marker == 0xe0 && is_jfif_app0(payload) {
            return true;
        }
        if opts.keep_icc && marker == 0xe2 && is_icc_app2_payload(payload) {
            return true;
        }
        return false; // all other APP
    }
    if is_sof(marker)
        || matches!(
            marker,
            0xc4 | 0xc8 | 0xcc | 0xdb | 0xdd
        ) {
        return true;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StripOptions;

    fn minimal_tiff() -> Vec<u8> {
        vec![b'I', b'I', 0x2a, 0, 8, 0, 0, 0, 0]
    }

    #[test]
    fn strips_app1_preserves_sos() {
        let tiff = minimal_tiff();
        let mut j = vec![0xff, 0xd8];
        let pl = (2 + 6 + tiff.len()) as u16;
        j.push(0xff);
        j.push(0xe1);
        j.extend_from_slice(&pl.to_be_bytes());
        j.extend_from_slice(b"Exif\0\0");
        j.extend_from_slice(&tiff);
        j.extend_from_slice(&[0xff, 0xda, 0, 8, 0, 0, 0, 0, 0, 0x3f, 0xff, 0xd9]);
        let s = strip(&j, &StripOptions::default()).unwrap();
        assert!(!s.windows(2).any(|w| w == [0xff, 0xe1]));
        assert!(s.len() < j.len());
    }
}
