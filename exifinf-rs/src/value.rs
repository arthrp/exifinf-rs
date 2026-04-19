use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    pub num: u32,
    pub den: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SRational {
    pub num: i32,
    pub den: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Rational(Rational),
    SRational(SRational),
    /// One or more rationals (e.g. GPS coordinate triplets).
    Rationals(Vec<Rational>),
    Ascii(String),
    Utf8(String),
    Undef(Vec<u8>),
    /// Homogeneous numeric arrays from multi-value TIFF fields.
    U16s(Vec<u16>),
    U32s(Vec<u32>),
    I16s(Vec<i16>),
    I32s(Vec<i32>),
    I8s(Vec<i8>),
    F32s(Vec<f32>),
    F64s(Vec<f64>),
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 0 {
            return write!(f, "{}/0", self.num);
        }
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl fmt::Display for SRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 0 {
            return write!(f, "{}/0", self.num);
        }
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::U8(v) => write!(f, "{v}"),
            Value::U16(v) => write!(f, "{v}"),
            Value::U32(v) => write!(f, "{v}"),
            Value::U64(v) => write!(f, "{v}"),
            Value::I8(v) => write!(f, "{v}"),
            Value::I16(v) => write!(f, "{v}"),
            Value::I32(v) => write!(f, "{v}"),
            Value::I64(v) => write!(f, "{v}"),
            Value::F32(v) => write!(f, "{v}"),
            Value::F64(v) => write!(f, "{v}"),
            Value::Rational(r) => write!(f, "{r}"),
            Value::SRational(r) => write!(f, "{r}"),
            Value::Rationals(rs) => {
                for (i, r) in rs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{r}")?;
                }
                Ok(())
            }
            Value::Ascii(s) | Value::Utf8(s) => write!(f, "{s}"),
            Value::Undef(b) => write!(f, "({} bytes)", b.len()),
            Value::U16s(v) => write_list(f, v),
            Value::U32s(v) => write_list(f, v),
            Value::I16s(v) => write_list(f, v),
            Value::I32s(v) => write_list(f, v),
            Value::I8s(v) => write_list(f, v),
            Value::F32s(v) => write_list(f, v),
            Value::F64s(v) => write_list(f, v),
        }
    }
}

fn write_list<T: fmt::Display>(f: &mut fmt::Formatter<'_>, v: &[T]) -> fmt::Result {
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }
        write!(f, "{x}")?;
    }
    Ok(())
}
