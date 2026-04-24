//! Strip metadata from ISO BMFF (MOV, MP4, HEIC) and fix sample/table offsets
//! when `mdat` moves.

use crate::error::{Error, Result};

const XMP_UUID: [u8; 16] = [
    0xBE, 0x7A, 0xCF, 0xCB, 0x97, 0xA9, 0x42, 0xE8, 0x9C, 0x71, 0x99, 0x94, 0x91, 0xE3, 0xAF, 0xAC,
];

struct BmffBox<'a> {
    kind: [u8; 4],
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
        s => (8, s as u64),
    };
    let end = at
        .checked_add(total as usize)
        .ok_or(Error::BadQt)?;
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

fn make_box(t: [u8; 4], body: Vec<u8>) -> Result<Vec<u8>> {
    let size = 8u32
        .checked_add(
            u32::try_from(body.len()).map_err(|_| Error::BadQt)?,
        )
        .ok_or(Error::BadQt)?;
    let mut v = vec![0u8; 8 + body.len()];
    v[0..4].copy_from_slice(&size.to_be_bytes());
    v[4..8].copy_from_slice(&t);
    v[8..].copy_from_slice(&body);
    Ok(v)
}

fn is_xmp(body: &[u8]) -> bool {
    body.len() >= 16 && body[0..16] == XMP_UUID[0..16]
}

fn is_heic_brand(brand: [u8; 4]) -> bool {
    matches!(
        &brand,
        b"heic" | b"heix" | b"mif1" | b"heis" | b"hevc" | b"hevx" | b"hevm" | b"hevs"
    )
}

/// Returns `(is_heic, major_brand_or_none)` from the first `ftyp` (skips `wide`, etc.).
fn ftyp_info_scan(data: &[u8]) -> (bool, Option<[u8; 4]>) {
    let mut p = 0usize;
    while p < data.len() {
        let (end, b) = match read_box(data, p) {
            Ok(Some(x)) => x,
            _ => break,
        };
        if b.kind == *b"ftyp" && b.body.len() >= 4 {
            let mut m = [0u8; 4];
            m.copy_from_slice(&b.body[0..4]);
            return (is_heic_brand(m), Some(m));
        }
        p = end;
    }
    (false, None)
}

/// Start offset of the first `mdat` *box* (its size field) in the file.
fn first_mdat_start(data: &[u8]) -> Option<usize> {
    let mut p = 0usize;
    while p < data.len() {
        match read_box(data, p) {
            Ok(Some((end, b))) => {
                if b.kind == *b"mdat" {
                    return Some(p);
                }
                p = end;
            }
            Ok(None) | Err(Error::Truncated) | Err(Error::BadQt) => break,
            Err(_) => break,
        }
    }
    None
}

fn has_moof(data: &[u8]) -> bool {
    let mut p = 0usize;
    while p < data.len() {
        if let Ok(Some((end, b))) = read_box(data, p) {
            if b.kind == *b"moof" {
                return true;
            }
            p = end;
        } else {
            break;
        }
    }
    false
}

/// Copy a sequence of sub-boxes byte-for-byte.
fn blit_children(data: &[u8], s: usize, e: usize) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut p = s;
    while p < e {
        let (end, _) = read_box(data, p)?.ok_or(Error::BadQt)?;
        if end > e {
            return Err(Error::BadQt);
        }
        out.extend_from_slice(&data[p..end]);
        p = end;
    }
    Ok(out)
}

fn filter_moov(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if end > e {
                return Err(Error::BadQt);
            }
            if b.kind == *b"udta" || b.kind == *b"meta" {
                r = end;
                continue;
            }
            if b.kind == *b"uuid" && is_xmp(b.body) {
                r = end;
                continue;
            }
            if b.kind == *b"trak" {
                o_.extend(filter_trak(d, r, end, o)?);
            } else {
                o_.extend_from_slice(&d[r..end]);
            }
            r = end;
        }
        o_
    };
    make_box(*b"moov", ch)
}

fn filter_trak(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if end > e {
                return Err(Error::BadQt);
            }
            if b.kind == *b"udta" || b.kind == *b"meta" {
                r = end;
                continue;
            }
            if b.kind == *b"uuid" && is_xmp(b.body) {
                r = end;
                continue;
            }
            if b.kind == *b"mdia" {
                o_.extend(filter_mdia(d, r, end, o)?);
            } else {
                o_.extend_from_slice(&d[r..end]);
            }
            r = end;
        }
        o_
    };
    make_box(*b"trak", ch)
}

fn filter_mdia(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if end > e {
                return Err(Error::BadQt);
            }
            if b.kind == *b"udta" || b.kind == *b"meta" {
                r = end;
                continue;
            }
            if b.kind == *b"minf" {
                o_.extend(filter_minf(d, r, end, o)?);
            } else {
                o_.extend_from_slice(&d[r..end]);
            }
            r = end;
        }
        o_
    };
    make_box(*b"mdia", ch)
}

fn filter_minf(d: &[u8], p: usize, e: usize, _o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        blit_children(d, inner, e)?
    };
    make_box(*b"minf", ch)
}

fn filter_ipco(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if b.kind == *b"colr" && !o.keep_icc {
                r = end;
                continue;
            }
            o_.extend_from_slice(&d[r..end]);
            r = end;
        }
        o_
    };
    make_box(*b"ipco", ch)
}

fn filter_iprp(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    let ch = {
        let inner = p + 8;
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if b.kind == *b"ipco" {
                o_.extend(filter_ipco(d, r, end, o)?);
            } else {
                o_.extend_from_slice(&d[r..end]);
            }
            r = end;
        }
        o_
    };
    make_box(*b"iprp", ch)
}

/// HEIC / HEIF `meta` at file level: keep structure, remove XMP `uuid` and (optional) colr
fn filter_top_meta(d: &[u8], p: usize, e: usize, o: &crate::StripOptions) -> Result<Vec<u8>> {
    if p + 8 + 4 > e {
        return Ok(d.get(p..e).ok_or(Error::BadQt)?.to_vec());
    }
    // FullBox: first 4 bytes of body
    let verf: [u8; 4] = d[(p + 8)..(p + 12)]
        .try_into()
        .map_err(|_| Error::BadQt)?;
    let inner = p + 12;
    let ch = {
        let mut o_ = vec![];
        let mut r = inner;
        while r < e {
            let (end, b) = read_box(d, r)?.ok_or(Error::BadQt)?;
            if b.kind == *b"meta" {
                r = end;
                continue;
            }
            if b.kind == *b"uuid" && is_xmp(b.body) {
                r = end;
                continue;
            }
            if b.kind == *b"iprp" {
                o_.extend(filter_iprp(d, r, end, o)?);
            } else {
                o_.extend_from_slice(&d[r..end]);
            }
            r = end;
        }
        o_
    };
    let mut body = verf.to_vec();
    body.extend_from_slice(&ch);
    make_box(*b"meta", body)
}

fn filter_top(d: &[u8], is_heic: bool, o: &crate::StripOptions) -> Result<Vec<u8>> {
    if has_moof(d) {
        return Err(Error::Unsupported("fragmented MP4 (moof)"));
    }
    let mut out = vec![];
    let mut p = 0usize;
    while p < d.len() {
        let (end, b) = read_box(d, p)?.ok_or(Error::BadQt)?;
        if b.kind == *b"moof" {
            return Err(Error::Unsupported("fragmented MP4 (moof)"));
        }
        if b.kind == *b"free" || b.kind == *b"skip" {
            p = end;
            continue;
        }
        if b.kind == *b"moov" {
            out.extend(filter_moov(d, p, end, o)?);
        } else if b.kind == *b"meta" {
            if is_heic {
                out.extend(filter_top_meta(d, p, end, o)?);
            } else {
                p = end;
                continue;
            }
        } else if b.kind == *b"uuid" && is_xmp(b.body) {
            p = end;
            continue;
        } else {
            out.extend_from_slice(&d[p..end]);
        }
        p = end;
    }
    Ok(out)
}

// --- post-process offsets ---

fn sub_u32(v: u32, d: u64) -> Result<u32> {
    if (v as u64) < d {
        return Err(Error::OffsetOverflow);
    }
    Ok((v as u64 - d) as u32)
}

fn sub_u64(v: u64, d: u64) -> Result<u64> {
    v.checked_sub(d).ok_or(Error::OffsetOverflow)
}

fn box_header_len(d: &[u8], p: usize) -> Result<usize> {
    if p + 8 > d.len() {
        return Err(Error::BadQt);
    }
    let s = u32::from_be_bytes(
        d[p..p + 4]
            .try_into()
            .map_err(|_| Error::BadQt)?,
    ) as u64;
    if s == 1 {
        Ok(16)
    } else {
        Ok(8)
    }
}

/// Inner `stco` / `co64` payload: FullBox, then n × offset.
fn patch_stco(d: &mut [u8], p: usize, e: usize, dlt: u64) -> Result<()> {
    let hl = box_header_len(d, p)?;
    let pl = p + hl;
    if pl + 8 > e {
        return Ok(());
    }
    let n = u32::from_be_bytes(
        d[pl + 4..pl + 8]
            .try_into()
            .map_err(|_| Error::BadQt)?,
    ) as usize;
    for i in 0..n {
        let o = 8 + i * 4;
        if pl + o + 4 > e {
            break;
        }
        let v = u32::from_be_bytes(
            d[pl + o..pl + o + 4]
                .try_into()
                .map_err(|_| Error::BadQt)?,
        );
        let n2 = sub_u32(v, dlt)?;
        d[pl + o..pl + o + 4].copy_from_slice(&n2.to_be_bytes());
    }
    Ok(())
}

fn patch_co64(d: &mut [u8], p: usize, e: usize, dlt: u64) -> Result<()> {
    let hl = box_header_len(d, p)?;
    let pl = p + hl;
    if pl + 8 > e {
        return Ok(());
    }
    let n = u32::from_be_bytes(
        d[pl + 4..pl + 8]
            .try_into()
            .map_err(|_| Error::BadQt)?,
    ) as usize;
    for i in 0..n {
        let o = 8 + i * 8;
        if pl + o + 8 > e {
            break;
        }
        let v = u64::from_be_bytes(
            d[pl + o..pl + o + 8]
                .try_into()
                .map_err(|_| Error::BadQt)?,
        );
        let n2 = sub_u64(v, dlt)?;
        d[pl + o..pl + o + 8].copy_from_slice(&n2.to_be_bytes());
    }
    Ok(())
}

/// Read 0/4/8-byte size field (FFmpeg `rb_size` semantics).
fn read_sized_n(body: &[u8], c: &mut usize, n: u8) -> Result<u64> {
    let v = match n {
        0 => 0u64,
        4 => {
            if *c + 4 > body.len() {
                return Err(Error::BadQt);
            }
            let o = u32::from_be_bytes(
                *<&[u8; 4]>::try_from(&body[*c..*c + 4])
                    .map_err(|_| Error::BadQt)?,
            ) as u64;
            *c += 4;
            o
        }
        8 => {
            if *c + 8 > body.len() {
                return Err(Error::BadQt);
            }
            let o = u64::from_be_bytes(
                *<&[u8; 8]>::try_from(&body[*c..*c + 8])
                    .map_err(|_| Error::BadQt)?,
            );
            *c += 8;
            o
        }
        _ => return Err(Error::Unsupported("iloc: odd size field width")),
    };
    Ok(v)
}

/// Write 0/4/8; `n==0` is no-op.
fn write_sized_n(body: &mut [u8], c: &mut usize, n: u8, v: u64) -> Result<()> {
    match n {
        0 => {}
        4 => {
            if *c + 4 > body.len() {
                return Err(Error::BadQt);
            }
            let w = u32::try_from(v).map_err(|_| Error::OffsetOverflow)?;
            body[*c..*c + 4].copy_from_slice(&w.to_be_bytes());
            *c += 4;
        }
        8 => {
            if *c + 8 > body.len() {
                return Err(Error::BadQt);
            }
            body[*c..*c + 8].copy_from_slice(&v.to_be_bytes());
            *c += 8;
        }
        _ => return Err(Error::Unsupported("iloc: odd size field width")),
    }
    Ok(())
}

/// Patch `iloc` in-place. Follows the layout parsed by FFmpeg `mov_read_iloc` (v0–1 only).
/// Adjusts the absolute file offset of each extent by subtracting `dlt` from `base + extent`
/// and writes back into base/extent (clears `base` when it was used).
fn patch_iloc(d: &mut [u8], p: usize, e: usize, dlt: u64) -> Result<()> {
    let hl = box_header_len(d, p)?;
    if p + hl > e {
        return Err(Error::BadQt);
    }
    let b = d.get_mut(p + hl..e).ok_or(Error::BadQt)?;
    if b.is_empty() {
        return Ok(());
    }
    if b[0] > 1 {
        return Err(Error::Unsupported("iloc: version not supported (need 0/1)"));
    }
    let version = b[0];
    let mut c: usize = 4;
    if c >= b.len() {
        return Ok(());
    }
    if b.len() < c + 1 {
        return Err(Error::BadQt);
    }
    let value = b[c];
    c += 1;
    let offset_size = (value >> 4) & 0x0F;
    let length_size = value & 0x0F;
    if c >= b.len() {
        return Err(Error::BadQt);
    }
    let value2 = b[c];
    c += 1;
    let base_offset_size = (value2 >> 4) & 0x0F;
    let index_size = if version == 0 {
        0u8
    } else {
        value2 & 0x0F
    };
    if index_size != 0 {
        return Err(Error::Unsupported("iloc: index_size != 0"));
    }
    if c + 2 > b.len() {
        return Err(Error::BadQt);
    }
    let item_count = u16::from_be_bytes(
        *<&[u8; 2]>::try_from(&b[c..c + 2])
            .map_err(|_| Error::BadQt)?,
    ) as usize;
    c += 2;
    for _ in 0..item_count {
        if version < 2 {
            if c + 2 > b.len() {
                return Err(Error::BadQt);
            }
            c += 2; // item_id
        } else {
            return Err(Error::Unsupported("iloc: v2+"));
        }
        let mut offset_type: u8 = 0;
        if version > 0 {
            if c + 2 > b.len() {
                return Err(Error::BadQt);
            }
            offset_type = (u16::from_be_bytes(
                *<&[u8; 2]>::try_from(&b[c..c + 2])
                    .map_err(|_| Error::BadQt)?,
            ) & 0x0F) as u8;
            c += 2;
        }
        if c + 2 > b.len() {
            return Err(Error::BadQt);
        }
        c += 2; // data_reference_index
        let base_start = c;
        let base = read_sized_n(b, &mut c, base_offset_size)?;
        if c + 2 > b.len() {
            return Err(Error::BadQt);
        }
        let extent_count = u16::from_be_bytes(
            *<&[u8; 2]>::try_from(&b[c..c + 2])
                .map_err(|_| Error::BadQt)?,
        ) as usize;
        c += 2;
        if extent_count > 1 {
            return Err(Error::Unsupported("iloc: extent_count > 1"));
        }
        for _ in 0..extent_count {
            let ext_start = c;
            let ext = read_sized_n(b, &mut c, offset_size)?;
            let _ext_len = read_sized_n(b, &mut c, length_size)?;
            if (version as u32) > 0 && offset_type == 1 {
                // idat-relative: do not change
                continue;
            }
            let abs = base
                .checked_add(ext)
                .ok_or(Error::BadQt)?;
            if abs < dlt {
                return Err(Error::OffsetOverflow);
            }
            let nabs = abs - dlt;
            if base_offset_size > 0 {
                let mut t = base_start;
                write_sized_n(b, &mut t, base_offset_size, 0)?;
            }
            let mut t2 = ext_start;
            write_sized_n(b, &mut t2, offset_size, nabs)?;
        }
    }
    Ok(())
}

fn is_container(t: [u8; 4]) -> bool {
    matches!(
        &t,
        b"moov" | b"trak" | b"edts" | b"tref" | b"mdia" | b"minf" | b"stbl" | b"dinf" | b"mvex"
            | b"iprp" | b"ipco" | b"udta" | b"sinf" | b"meta"
    )
}

/// Child box area for `read_box` iteration (assumes 8- or 16-byte header, non-extended size).
fn container_inner(p: usize, d: &[u8], e: usize, kind: [u8; 4]) -> Result<usize> {
    let h = box_header_len(d, p)?;
    if kind == *b"meta" || kind == *b"mvex" {
        if p + h + 4 > e {
            return Err(Error::BadQt);
        }
        return Ok(p + h + 4);
    }
    if p + h > e {
        return Err(Error::BadQt);
    }
    Ok(p + h)
}

fn rec_patch(d: &mut [u8], s: usize, e: usize, dlt: u64) -> Result<()> {
    let mut p = s;
    while p < e {
        let (end, b) = read_box(&*d, p)?.ok_or(Error::BadQt)?;
        if end > e {
            return Err(Error::BadQt);
        }
        if b.kind == *b"stco" {
            patch_stco(d, p, end, dlt)?;
        } else if b.kind == *b"co64" {
            patch_co64(d, p, end, dlt)?;
        } else if b.kind == *b"iloc" {
            patch_iloc(d, p, end, dlt)?;
        } else if is_container(b.kind) {
            let is = container_inner(p, d, end, b.kind)?;
            if is < end {
                rec_patch(d, is, end, dlt)?;
            }
        }
        p = end;
    }
    Ok(())
}

fn post_patch(d: &mut [u8], dlt: u64) -> Result<()> {
    if dlt == 0 {
        return Ok(());
    }
    rec_patch(d, 0, d.len(), dlt)
}

pub fn strip(data: &[u8], opts: &crate::StripOptions) -> Result<Vec<u8>> {
    let (heic, _) = ftyp_info_scan(data);
    let old = first_mdat_start(data);
    let mut out = filter_top(data, heic, opts)?;
    let new = first_mdat_start(&out);
    let dlt = match (old, new) {
        (Some(a), Some(b)) => a as u64 - b as u64,
        _ => 0u64,
    };
    post_patch(&mut out, dlt)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_box_size() {
        let b = make_box(*b"ftyp", b"x".to_vec()).unwrap();
        assert_eq!(&b[0..4], &9u32.to_be_bytes());
        assert_eq!(&b[4..8], b"ftyp");
        assert_eq!(b[8], b'x');
    }
}
