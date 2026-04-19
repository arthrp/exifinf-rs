# exifinf-rs

Partial Rust port of [ExifTool](https://exiftool.org/) utility. Read-only EXIF metadata extraction in Rust for **JPEG**, **TIFF**, and **PNG**. Tag naming and formatting follow a **subset of [ExifTool](https://exiftool.org/) semantics**, so output should feel familiar if you use that tool elsewhere.

## Workspace

| Crate | Role |
|--------|------|
| `exifinf-rs` | Library: `extract`, `extract_from_path`, typed values, and printable tag records |
| `exifinf-cli` | `exifinf` binary for quick inspection from the shell |

## CLI

```sh
cargo run -p exifinf-cli -- path/to/image.jpg
```

Prints one line per tag: `[group] name = value`.

## Library

```rust
use std::path::Path;
use exifinf_rs::{extract_from_path, format_record};

let meta = extract_from_path(Path::new("photo.jpg"))?;
for t in &meta.tags {
    println!("{} = {}", t.name, format_record(t, &meta.tags));
}
```

