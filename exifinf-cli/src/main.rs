use std::env;
use std::path::Path;
use std::process;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: exifinf <file>");
        process::exit(2);
    });
    let meta = match exifinf_rs::extract_from_path(Path::new(&path)) {
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
