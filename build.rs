use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use flate2::Compression;
use flate2::write::GzEncoder;

fn main() {
    embuild::espidf::sysenv::output();
    let out_dir = env::var("OUT_DIR").unwrap();
    let files = ["index.html","style.css","script.js"];
    for file in files {
        let input_path = format!("frontend/{}", file);
        println!("cargo:rerun-if-changed={}", input_path);
        let mut input = File::open(&input_path).unwrap();
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer).unwrap();
        let output_path = Path::new(&out_dir).join(format!("{}.gz", file));
        let output = File::create(output_path).unwrap();
        let mut encoder = GzEncoder::new(output, Compression::best());
        encoder.write_all(&buffer).unwrap();
        encoder.finish().unwrap();
    }
}