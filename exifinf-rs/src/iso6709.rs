//! Parse ISO 6709 geographic point strings (subset used in QuickTime `©xyz` / location).

/// Returns (latitude °, longitude °, altitude m or None).
pub fn parse_iso6709(s: &str) -> Option<(f64, f64, Option<f64>)> {
    let s = s.trim().trim_end_matches('/').trim();
    if s.is_empty() {
        return None;
    }
    let mut i = 0usize;
    let lat = parse_signed_number(s, &mut i)?;
    if i >= s.len() {
        return None;
    }
    // Longitude: next signed number, e.g. `...-122.08...` or `...+151.0...` (keep leading sign).
    let lon = parse_signed_number(s, &mut i)?;
    let alt = if i < s.len() {
        if s.as_bytes()[i] != b'+' && s.as_bytes()[i] != b'-' {
            return None;
        }
        Some(parse_signed_number(s, &mut i)?)
    } else {
        None
    };
    if i != s.len() {
        return None;
    }
    Some((lat, lon, alt))
}

fn parse_signed_number(s: &str, i: &mut usize) -> Option<f64> {
    let b = s.as_bytes().get(*i)?;
    let sign = match *b {
        b'+' => {
            *i += 1;
            1.0
        }
        b'-' => {
            *i += 1;
            -1.0
        }
        _ => 1.0,
    };
    let start = *i;
    while *i < s.len() {
        let c = s.as_bytes()[*i];
        if c == b'.' || c.is_ascii_digit() {
            *i += 1;
        } else {
            break;
        }
    }
    if *i == start {
        return None;
    }
    let frag = s.get(start..*i)?;
    let v: f64 = frag.parse().ok()?;
    Some(sign * v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_example() {
        let (la, lo, al) = parse_iso6709("+37.4221-122.0841+030.000/").unwrap();
        assert!((la - 37.4221).abs() < 1e-6);
        assert!((lo - (-122.0841)).abs() < 1e-6);
        assert!((al.unwrap() - 30.0).abs() < 1e-6);
    }
}
