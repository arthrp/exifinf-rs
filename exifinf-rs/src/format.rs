/// EXIF/TIFF format type numbers (TIFF spec + ExifTool extras).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Format {
    Int8u = 1,
    String = 2,
    Int16u = 3,
    Int32u = 4,
    Rational64u = 5,
    Int8s = 6,
    Undef = 7,
    Int16s = 8,
    Int32s = 9,
    Rational64s = 10,
    Float = 11,
    Double = 12,
    Ifd = 13,
    Unicode = 14,
    Complex = 15,
    Int64u = 16,
    Int64s = 17,
    Ifd64 = 18,
    Utf8 = 129,
}

impl Format {
    pub fn size(self) -> usize {
        match self {
            Format::Int8u | Format::String | Format::Int8s | Format::Undef | Format::Utf8 => 1,
            Format::Int16u | Format::Int16s => 2,
            Format::Int32u
            | Format::Int32s
            | Format::Float
            | Format::Ifd
            | Format::Unicode
            | Format::Complex => 4,
            Format::Rational64u | Format::Rational64s => 8,
            Format::Double | Format::Int64u | Format::Int64s | Format::Ifd64 => 8,
        }
    }

    pub fn from_u16(n: u16) -> Option<Format> {
        Some(match n {
            1 => Format::Int8u,
            2 => Format::String,
            3 => Format::Int16u,
            4 => Format::Int32u,
            5 => Format::Rational64u,
            6 => Format::Int8s,
            7 => Format::Undef,
            8 => Format::Int16s,
            9 => Format::Int32s,
            10 => Format::Rational64s,
            11 => Format::Float,
            12 => Format::Double,
            13 => Format::Ifd,
            14 => Format::Unicode,
            15 => Format::Complex,
            16 => Format::Int64u,
            17 => Format::Int64s,
            18 => Format::Ifd64,
            129 => Format::Utf8,
            _ => return None,
        })
    }
}
