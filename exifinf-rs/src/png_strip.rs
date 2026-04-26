use crate::error::{Error, Result};
use crate::common::{PNG_SIG};


/// CRC-32 (IEEE) for chunk type+data
fn chunk_crc32(typ: [u8; 4], data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB88320u32 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            t[i] = c;
            i += 1;
        }
        t
    };
    // PNG (ISO 3309): init all 1s, final complement (same as libpng)
    let mut c: u32 = 0xFFFF_FFFF;
    for &x in &typ {
        c = TABLE[((c ^ u32::from(x)) as u8) as usize] ^ (c >> 8);
    }
    for &x in data {
        c = TABLE[((c ^ u32::from(x)) as u8) as usize] ^ (c >> 8);
    }
    !c
}

fn is_critical_chunk(typ: [u8; 4]) -> bool {
    typ[0].is_ascii_uppercase()
}

/// Return true if this chunk is kept when stripping.
pub fn keep_chunk(typ: [u8; 4], opts: &crate::StripOptions) -> bool {
    match &typ {
        b"IHDR" | b"PLTE" | b"IDAT" | b"IEND" => true,
        b"gAMA" | b"cHRM" | b"sRGB" | b"tRNS" | b"bKGD" => opts.keep_color_info,
        b"iCCP" => opts.keep_icc,
        b"eXIf" | b"zxIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"tIME" | b"pHYs" | b"sPLT" | b"hIST" | b"sBIT" => false,
        t if t[0] == b'X' && t[1] | 32 == b'm' && t[2] | 32 == b'p' => false, // strip vendor XmP* / XMP
        _ if is_critical_chunk(typ) => true, // keep unknown critical chunks
        _ => false, // strip unknown ancillary
    }
}

/// Strip metadata chunks from a PNG, recomputing CRC-32.
pub fn strip(data: &[u8], opts: &crate::StripOptions) -> Result<Vec<u8>> {
    if data.len() < 8 {
        return Err(Error::BadPng);
    }
    if data[0..8] != PNG_SIG[..] {
        return Err(Error::BadPng);
    }
    let mut out: Vec<u8> = data[0..8].to_vec();
    let mut p = 8usize;
    let mut has_ihdr = false;
    let mut has_idat = false;
    let mut has_iend = false;
    while p + 8 <= data.len() {
        let len = u32::from_be_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]) as usize;
        let mut chunk_type = [0u8; 4];
        chunk_type.copy_from_slice(&data[p + 4..p + 8]);
        let dstart = p + 8;
        let dend = dstart
            .checked_add(len)
            .ok_or(Error::BadPng)?;
        if dend + 4 > data.len() {
            return Err(Error::Truncated);
        }
        let chunk_data = &data[dstart..dend];
        let stored_crc = u32::from_be_bytes([data[dend], data[dend + 1], data[dend + 2], data[dend + 3]]);

        p = dend + 4;
        if chunk_crc32(chunk_type, chunk_data) != stored_crc {
            return Err(Error::BadPng);
        }

        if !keep_chunk(chunk_type, opts) {
            continue;
        }
        if chunk_type == *b"IHDR" {
            has_ihdr = true;
        }
        if chunk_type == *b"IDAT" {
            has_idat = true;
        }
        if chunk_type == *b"IEND" {
            has_iend = true;
        }
        let len_u32: u32 = u32::try_from(len).map_err(|_| Error::BadPng)?;
        out.extend_from_slice(&len_u32.to_be_bytes());
        out.extend_from_slice(&chunk_type);
        out.extend_from_slice(chunk_data);
        out.extend_from_slice(&chunk_crc32(chunk_type, chunk_data).to_be_bytes());
    }
    if !has_ihdr || !has_idat || !has_iend {
        return Err(Error::BadPng);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_default_strips_ancillary_name() {
        assert!(!keep_chunk(*b"eXIf", &crate::StripOptions::default()));
        assert!(keep_chunk(*b"IHDR", &crate::StripOptions::default()));
    }
}
