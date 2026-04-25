use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    UnknownFormat,
    Truncated,
    BadMagic,
    BadTiff,
    BadPng,
    BadJpeg,
    BadQuicktime,
    Decompress(String),
    Unsupported(&'static str),
    /// stco / co64 / iloc underflow when adjusting for removed bytes
    OffsetOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::UnknownFormat => write!(f, "unknown image format"),
            Error::Truncated => write!(f, "truncated data"),
            Error::BadMagic => write!(f, "invalid file signature"),
            Error::BadTiff => write!(f, "invalid TIFF structure"),
            Error::BadPng => write!(f, "invalid PNG structure"),
            Error::BadJpeg => write!(f, "invalid JPEG structure"),
            Error::BadQuicktime => write!(f, "invalid QuickTime/MP4 (ISO BMFF) structure"),
            Error::Decompress(s) => write!(f, "decompression failed: {s}"),
            Error::Unsupported(s) => write!(f, "unsupported: {s}"),
            Error::OffsetOverflow => write!(f, "offset adjustment would underflow (invalid BMFF)"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
