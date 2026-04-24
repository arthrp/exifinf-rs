use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process;

use exifinf_rs::StripOptions;

fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    if a.is_empty() {
        eprintln!(
            "usage: exifinf [--strip] [-o <outfile>] [--overwrite-original] [--keep-icc] [--keep-color] [--keep-jfif] <file>"
        );
        eprintln!("       exifinf <file>   # list tags");
        process::exit(2);
    }

    let mut i = 0usize;
    let mut strip = false;
    let mut out_path: Option<String> = None;
    let mut op = StripOptions::default();
    let mut in_path: Option<String> = None;
    while i < a.len() {
        match a[i].as_str() {
            "--strip" => {
                strip = true;
                i += 1;
            }
            "-o" | "--output" => {
                let f = a.get(i + 1).cloned();
                if f.is_none() {
                    eprintln!("error: {}/--output needs a path", "-o");
                    process::exit(2);
                }
                out_path = f;
                i += 2;
            }
            "--overwrite-original" => {
                op.overwrite_original = true;
                i += 1;
            }
            "--keep-icc" => {
                op.keep_icc = true;
                i += 1;
            }
            "--keep-color" | "--keep-color-info" => {
                op.keep_color_info = true;
                i += 1;
            }
            "--keep-jfif" => {
                op.keep_jfif = true;
                i += 1;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option {s}");
                process::exit(2);
            }
            _ => {
                if in_path.is_some() {
                    eprintln!("error: only one input file is supported");
                    process::exit(2);
                }
                in_path = Some(a[i].clone());
                i += 1;
            }
        }
    }

    let in_path = in_path.unwrap_or_else(|| {
        eprintln!("error: missing <file>");
        process::exit(2);
    });
    let p = Path::new(&in_path);

    if strip {
        if let Some(o) = out_path {
            let b = match fs::read(p) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            let n = match exifinf_rs::strip_metadata(&b, &op) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            if let Err(e) = (|| -> std::io::Result<()> {
                let mut f = fs::File::create(&o)?;
                f.write_all(&n)?;
                f.sync_all()?;
                Ok(())
            })() {
                eprintln!("{e}");
                process::exit(1);
            }
            eprintln!(
                "stripped {} -> {} ({} -> {} bytes)",
                in_path,
                o,
                b.len(),
                n.len()
            );
        } else {
            if let Err(e) = exifinf_rs::strip_metadata_in_place(p, &op) {
                eprintln!("{e}");
                process::exit(1);
            }
            let b = match fs::read(p) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };
            eprintln!("stripped {in_path} in place ({} bytes)", b.len());
        }
        return;
    }

    if out_path.is_some() {
        eprintln!("error: -o requires --strip");
        process::exit(2);
    }
    if op.overwrite_original || op.keep_icc || op.keep_color_info || op.keep_jfif {
        eprintln!("error: these options require --strip");
        process::exit(2);
    }

    let meta = match exifinf_rs::extract_from_path(p) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };
    let tags = &meta.tags;
    for t in &meta.tags {
        let v = exifinf_rs::format_record(t, tags);
        println!("[{}] {} = {}", t.group, t.name, v);
    }
}
