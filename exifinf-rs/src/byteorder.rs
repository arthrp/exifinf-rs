use crate::error::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

pub fn read_u16(data: &[u8], off: usize, e: Endian) -> Result<u16> {
    let b = data.get(off..off + 2).ok_or(Error::Truncated)?;
    Ok(match e {
        Endian::Little => u16::from_le_bytes([b[0], b[1]]),
        Endian::Big => u16::from_be_bytes([b[0], b[1]]),
    })
}

pub fn read_i16(data: &[u8], off: usize, e: Endian) -> Result<i16> {
    Ok(read_u16(data, off, e)? as i16)
}

pub fn read_u32(data: &[u8], off: usize, e: Endian) -> Result<u32> {
    let b = data.get(off..off + 4).ok_or(Error::Truncated)?;
    Ok(match e {
        Endian::Little => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        Endian::Big => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
    })
}

pub fn read_i32(data: &[u8], off: usize, e: Endian) -> Result<i32> {
    Ok(read_u32(data, off, e)? as i32)
}

pub fn read_u64(data: &[u8], off: usize, e: Endian) -> Result<u64> {
    let b = data.get(off..off + 8).ok_or(Error::Truncated)?;
    Ok(match e {
        Endian::Little => u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]),
        Endian::Big => u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]),
    })
}

pub fn read_i64(data: &[u8], off: usize, e: Endian) -> Result<i64> {
    Ok(read_u64(data, off, e)? as i64)
}

pub fn read_f32(data: &[u8], off: usize, e: Endian) -> Result<f32> {
    Ok(f32::from_bits(read_u32(data, off, e)?))
}

pub fn read_f64(data: &[u8], off: usize, e: Endian) -> Result<f64> {
    Ok(f64::from_bits(read_u64(data, off, e)?))
}
