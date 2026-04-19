use crate::gps;
use crate::metadata::TagRecord;
use crate::tag_def::PrintConv;
use crate::tables::{lookup_exif, lookup_gps, lookup_png_text_by_tagname};
use crate::value::Value;

fn value_as_i64(v: &Value) -> Option<i64> {
    Some(match v {
        Value::U8(x) => i64::from(*x),
        Value::U16(x) => i64::from(*x),
        Value::U32(x) => i64::from(*x),
        Value::U64(x) => i64::try_from(*x).ok()?,
        Value::I8(x) => i64::from(*x),
        Value::I16(x) => i64::from(*x),
        Value::I32(x) => i64::from(*x),
        Value::I64(x) => *x,
        _ => return None,
    })
}

fn apply_map(pc: PrintConv, v: &Value) -> Option<String> {
    match pc {
        PrintConv::None => None,
        PrintConv::IntMap(pairs) => {
            let n = value_as_i64(v)?;
            pairs
                .binary_search_by_key(&n, |p| p.0)
                .ok()
                .map(|i| pairs[i].1.to_string())
        }
        PrintConv::StrMap(pairs) => {
            let key = match v {
                Value::Ascii(s) | Value::Utf8(s) => s.as_str(),
                _ => return None,
            };
            let key = key.trim();
            pairs
                .binary_search_by(|p| p.0.cmp(key))
                .ok()
                .map(|i| pairs[i].1.to_string())
        }
    }
}

fn sibling_value<'a>(records: &'a [TagRecord], group: &str, name: &str) -> Option<&'a Value> {
    records
        .iter()
        .find(|t| t.group == group && t.name == name)
        .map(|t| &t.value)
}

pub fn format_record(rec: &TagRecord, all: &[TagRecord]) -> String {
    if rec.group == "GPS" {
        if rec.name == "GPSLatitude" {
            let pref = sibling_value(all, "GPS", "GPSLatitudeRef");
            if let Some(s) = gps::format_coordinate(&rec.value, pref.and_then(|v| match v {
                Value::Ascii(s) | Value::Utf8(s) => Some(s.as_str()),
                _ => None,
            }), true)
            {
                return s;
            }
        }
        if rec.name == "GPSLongitude" {
            let pref = sibling_value(all, "GPS", "GPSLongitudeRef");
            if let Some(s) = gps::format_coordinate(&rec.value, pref.and_then(|v| match v {
                Value::Ascii(s) | Value::Utf8(s) => Some(s.as_str()),
                _ => None,
            }), false)
            {
                return s;
            }
        }
        if rec.name == "GPSLatitudeRef" {
            if let Some(s) = gps::format_lat_ref(&rec.value) {
                return s;
            }
        }
        if rec.name == "GPSLongitudeRef" {
            if let Some(s) = gps::format_lon_ref(&rec.value) {
                return s;
            }
        }
    }

    if let Some(id) = rec.tag_id {
        let in_gps = rec.group == "GPS";
        let def = if in_gps {
            lookup_gps(id)
        } else {
            lookup_exif(id)
        };
        if let Some(d) = def {
            if let Some(s) = apply_map(d.print_conv, &rec.value) {
                return s;
            }
        }
    } else if let Some(d) = lookup_png_text_by_tagname(&rec.name) {
        if rec.group == d.group1 || rec.group == "PNG" {
            if let Some(s) = apply_map(d.print_conv, &rec.value) {
                return s;
            }
        }
    }

    rec.value.to_string()
}
