use crate::value::Value;

#[derive(Clone, Debug)]
pub struct TagRecord {
    pub group: String,
    pub name: String,
    /// TIFF/EXIF tag id when known (used for PrintConv lookup).
    pub tag_id: Option<u16>,
    pub value: Value,
}

#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub tags: Vec<TagRecord>,
}

impl Metadata {
    pub fn push(&mut self, group: impl Into<String>, name: impl Into<String>, value: Value) {
        self.tags.push(TagRecord {
            group: group.into(),
            name: name.into(),
            tag_id: None,
            value,
        });
    }

    pub fn push_id(
        &mut self,
        group: impl Into<String>,
        name: impl Into<String>,
        tag_id: u16,
        value: Value,
    ) {
        self.tags.push(TagRecord {
            group: group.into(),
            name: name.into(),
            tag_id: Some(tag_id),
            value,
        });
    }
}
