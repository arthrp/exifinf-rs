//! WASM bindings for `exifinf-rs` (browser / Node via wasm-bindgen).

use exifinf_rs::{StripOptions, Value, extract as core_extract, format_record, strip_metadata as core_strip};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Deserialize)]
struct WasmStripOptions {
    #[serde(default)]
    keep_icc: bool,
    #[serde(default)]
    keep_color_info: bool,
    #[serde(default)]
    keep_jfif: bool,
    #[serde(default)]
    overwrite_original: bool,
}

impl From<WasmStripOptions> for StripOptions {
    fn from(o: WasmStripOptions) -> Self {
        StripOptions {
            keep_icc: o.keep_icc,
            keep_color_info: o.keep_color_info,
            keep_jfif: o.keep_jfif,
            overwrite_original: o.overwrite_original,
        }
    }
}

#[derive(Serialize)]
struct TagOut<'a> {
    group: &'a str,
    name: &'a str,
    tag_id: Option<u16>,
    display: String,
    value: serde_json::Value,
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::U8(n) => serde_json::Value::from(*n),
        Value::U16(n) => serde_json::Value::from(*n),
        Value::U32(n) => serde_json::Value::from(*n),
        Value::U64(n) => serde_json::Value::from(*n),
        Value::I8(n) => serde_json::Value::from(*n),
        Value::I16(n) => serde_json::Value::from(*n),
        Value::I32(n) => serde_json::Value::from(*n),
        Value::I64(n) => serde_json::Value::from(*n),
        Value::F32(n) => serde_json::json!(*n),
        Value::F64(n) => serde_json::json!(*n),
        Value::Rational(r) => serde_json::json!({ "num": r.num, "den": r.den }),
        Value::SRational(r) => serde_json::json!({ "num": r.num, "den": r.den }),
        Value::Rationals(rs) => serde_json::Value::Array(
            rs.iter()
                .map(|r| serde_json::json!({ "num": r.num, "den": r.den }))
                .collect(),
        ),
        Value::Ascii(s) | Value::Utf8(s) => serde_json::Value::String(s.clone()),
        Value::Undef(b) => serde_json::Value::Array(b.iter().map(|&x| serde_json::Value::from(x)).collect()),
        Value::U16s(v) => serde_json::Value::Array(v.iter().map(|&x| x.into()).collect()),
        Value::U32s(v) => serde_json::Value::Array(v.iter().map(|&x| x.into()).collect()),
        Value::I16s(v) => serde_json::Value::Array(v.iter().map(|&x| x.into()).collect()),
        Value::I32s(v) => serde_json::Value::Array(v.iter().map(|&x| x.into()).collect()),
        Value::I8s(v) => serde_json::Value::Array(v.iter().map(|&x| x.into()).collect()),
        Value::F32s(v) => serde_json::Value::Array(v.iter().map(|&x| serde_json::json!(x)).collect()),
        Value::F64s(v) => serde_json::Value::Array(v.iter().map(|&x| serde_json::json!(x)).collect()),
    }
}

/// Extract metadata from image bytes. Returns a JSON array of tag objects.
#[wasm_bindgen]
pub fn extract(bytes: &[u8]) -> Result<JsValue, JsError> {
    let meta = core_extract(bytes).map_err(|e| JsError::new(&e.to_string()))?;
    let out: Vec<TagOut> = meta
        .tags
        .iter()
        .map(|t| TagOut {
            group: &t.group,
            name: &t.name,
            tag_id: t.tag_id,
            display: format_record(t, &meta.tags),
            value: value_to_json(&t.value),
        })
        .collect();
    serde_wasm_bindgen::to_value(&out).map_err(|e| JsError::new(&e.to_string()))
}

/// Remove metadata from image bytes. `opts` is optional JSON (`keep_icc`, `keep_color_info`, `keep_jfif`, `overwrite_original`).
#[wasm_bindgen]
pub fn strip_metadata(bytes: &[u8], opts: JsValue) -> Result<Vec<u8>, JsError> {
    let strip_opts: StripOptions = if opts.is_undefined() || opts.is_null() {
        StripOptions::default()
    } else {
        let wasm_opts: WasmStripOptions =
            serde_wasm_bindgen::from_value(opts).map_err(|e| JsError::new(&e.to_string()))?;
        wasm_opts.into()
    };
    core_strip(bytes, &strip_opts).map_err(|e| JsError::new(&e.to_string()))
}
