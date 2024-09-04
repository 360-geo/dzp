use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{PathBuf};
use std::time::Instant;
use clap::Parser;
use dzp::{DzpConverter};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Source path for the .jpg files
    #[arg(short,long)]
    input_path: PathBuf,

    /// Destination path for the .dzp files
    #[arg(short,long)]
    output_path: PathBuf,
}

pub fn main() {
    let args = Args::parse();

    // step 1: find all .jpg files in args.input_dir and iterate over them
    let jpg_files: Vec<_> = fs::read_dir(&args.input_path)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().unwrap_or_default() == "jpg" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let mut dzp_converter = DzpConverter::create();

    for jpg_path in jpg_files {
        let now = Instant::now();

        let dzp_path = args.output_path.join(jpg_path.with_extension("dzp").file_name().unwrap());

        let image = image::open(jpg_path).unwrap();

        let dzp_bytes = dzp_converter.convert_image(&image);

        let mut dzp = File::create(&dzp_path).unwrap();
        dzp.write_all(&dzp_bytes).unwrap();

        let elapsed = now.elapsed();

        println!("Created {} in {:.3}s", dzp_path.file_name().unwrap().to_string_lossy(), elapsed.as_secs_f64());
    }
}
