use crate::format::Format;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintConv {
    None,
    IntMap(&'static [(i64, &'static str)]),
    StrMap(&'static [(&'static str, &'static str)]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubDir {
    None,
    ExifIfd,
    GpsIfd,
    InteropIfd,
    SubIfd,
    MakerNotes,
}

#[derive(Clone, Copy, Debug)]
pub struct TagDef {
    pub name: &'static str,
    #[allow(dead_code)]
    pub format: Option<Format>,
    pub print_conv: PrintConv,
    pub sub_dir: SubDir,
    #[allow(dead_code)]
    pub group1: &'static str,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct PngChunkDef {
    pub chunk_name: &'static str,
    pub group1: &'static str,
    pub print_conv: PrintConv,
}

#[derive(Clone, Copy, Debug)]
pub struct PngTextDef {
    pub name: &'static str,
    pub group1: &'static str,
    pub print_conv: PrintConv,
}
