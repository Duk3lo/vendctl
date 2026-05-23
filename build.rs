use std::env;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use flate2::write::GzEncoder;
use flate2::Compression;

fn compress_file(input_path: &Path, output_path: &Path) {
    let mut input = File::open(input_path).unwrap();
    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer).unwrap();
    let output = File::create(output_path).unwrap();
    let mut encoder = GzEncoder::new(output, Compression::best());
    encoder.write_all(&buffer).unwrap();
    encoder.finish().unwrap();
    println!("Compressed: {}", input_path.display());
}

fn process_dir(dir: &Path, out_dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            process_dir(&path, out_dir);
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path.strip_prefix("frontend").unwrap();
        let output_path = out_dir.join(format!("{}.gz", relative.display()));
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        compress_file(&path, output_path.as_path());
    }
}

fn main() {
    embuild::espidf::sysenv::output();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let frontend_dir = Path::new("frontend");
    process_dir(frontend_dir, &out_dir);
}
