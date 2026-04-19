use crate::value::{Rational, Value};

/// Convert EXIF GPS rational triplets to decimal degrees (ExifTool `ToDegrees`-style).
pub fn format_coordinate(val: &Value, ref_dir: Option<&str>, _is_lat: bool) -> Option<String> {
    let rats: &[Rational] = match val {
        Value::Rationals(r) if r.len() >= 3 => r.as_slice(),
        Value::Rational(r) => std::slice::from_ref(r),
        _ => return None,
    };
    let d = rat_to_f64(&rats[0])?;
    let m = rat_to_f64(&rats[1])?;
    let s = rat_to_f64(&rats[2])?;
    let mut deg = d + (m + s / 60.0) / 60.0;
    let neg = ref_dir
        .map(|r| {
            let u = r.trim().to_ascii_uppercase();
            u.starts_with('S') || u.starts_with('W')
        })
        .unwrap_or(false);
    if neg {
        deg = -deg.abs();
    }
    Some(format!("{deg:.6}"))
}

fn rat_to_f64(r: &Rational) -> Option<f64> {
    if r.den == 0 {
        return None;
    }
    Some(f64::from(r.num) / f64::from(r.den))
}

pub fn format_lat_ref(val: &Value) -> Option<String> {
    let c = ascii_first(val)?;
    match c {
        'N' | 'n' => Some("North".into()),
        'S' | 's' => Some("South".into()),
        _ => None,
    }
}

pub fn format_lon_ref(val: &Value) -> Option<String> {
    let c = ascii_first(val)?;
    match c {
        'E' | 'e' => Some("East".into()),
        'W' | 'w' => Some("West".into()),
        _ => None,
    }
}

fn ascii_first(val: &Value) -> Option<char> {
    let s = match val {
        Value::Ascii(x) | Value::Utf8(x) => x.as_str(),
        _ => return None,
    };
    s.trim().chars().next()
}
