//! Generated tag tables (`OUT_DIR/tables.rs`) are included here.

include!(concat!(env!("OUT_DIR"), "/tables.rs"));

pub fn lookup_exif(id: u16) -> Option<&'static crate::tag_def::TagDef> {
    EXIF_MAIN
        .binary_search_by_key(&id, |p| p.0)
        .ok()
        .map(|i| &EXIF_MAIN[i].1)
}

pub fn lookup_gps(id: u16) -> Option<&'static crate::tag_def::TagDef> {
    GPS_MAIN
        .binary_search_by_key(&id, |p| p.0)
        .ok()
        .map(|i| &GPS_MAIN[i].1)
}

pub fn lookup_png_text(keyword: &str) -> Option<&'static crate::tag_def::PngTextDef> {
    PNG_TEXTUAL
        .binary_search_by_key(&keyword, |p| p.0)
        .ok()
        .map(|i| &PNG_TEXTUAL[i].1)
}

pub fn lookup_png_text_by_tagname(name: &str) -> Option<&'static crate::tag_def::PngTextDef> {
    PNG_TEXTUAL
        .iter()
        .find(|(_, d)| d.name == name)
        .map(|(_, d)| d)
}
