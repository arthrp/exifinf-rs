# exifinf-rs

Partial Rust port of [ExifTool](https://exiftool.org/) utility. Read-only metadata extraction in Rust for **JPEG**, **TIFF**, and **PNG**, **QuickTime / ISO BMFF** containers (**MOV/MP4/M4A/HEIC** and other). Tag naming and formatting follow a **subset of [ExifTool](https://exiftool.org/) semantics**, so output should feel familiar if you use that tool elsewhere.

## Workspace

| Crate | Role |
|--------|------|
| `exifinf-rs` | Library: `extract`, `extract_from_path`, `strip_metadata`, … |
| `exifinf-cli` | `exifinf` binary for quick inspection (and optional stripping) from the shell |

## CLI

```sh
cargo run -p exifinf-cli -- path/to/photo.jpg
cargo run -p exifinf-cli -- path/to/video.mov
cargo run -p exifinf-cli -- path/to/photo.heic
# Strip metadata in memory and write a new file (or use --overwrite-original)
cargo run -p exifinf-cli -- --strip -o /tmp/clean.jpg path/to/photo.jpg
```

Prints one line per tag: `[group] name = value`.

`--strip` writes stripped bytes to `-o` / `--output`, or updates the file in place (with a `path_original` backup unless `--overwrite-original`). Use `--keep-icc`, `--keep-color` / `--keep-color-info`, or `--keep-jfif` to retain some auxiliary chunks/segments. **TIFF** stripping is not supported (returns an error). **HEIC** / **MP4** / **MOV** strip removes extra metadata boxes where safe; some fragmented or unusual BMFF layouts may be rejected.

## Library

```rust
use std::path::Path;
use exifinf_rs::{extract_from_path, format_record};

let meta = extract_from_path(Path::new("photo.jpg"))?; // also MOV, MP4, HEIC, …
for t in &meta.tags {
    println!("{} = {}", t.name, format_record(t, &meta.tags));
}
```

`strip_metadata` and `strip_metadata_in_place` accept `StripOptions` (same keep flags and `overwrite_original` as the CLI) and support **JPEG**, **PNG**, and **QuickTime/MP4/HEIC**-style files.

