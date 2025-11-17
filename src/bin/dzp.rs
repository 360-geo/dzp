use clap::Parser;
use dzp::DzpConverter;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Source path for the .jpg files
    #[arg(short, long)]
    input_path: PathBuf,

    /// Destination path for the .dzp files
    #[arg(short, long)]
    output_path: PathBuf,
}

pub fn main() {
    let args = Args::parse();

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

    let dzp_converter = DzpConverter::create();

    fs::create_dir_all(&args.output_path).unwrap();

    for jpg_path in jpg_files {
        let now = Instant::now();

        let dzp_path = args
            .output_path
            .join(jpg_path.with_extension("dzp").file_name().unwrap());

        let img = image::open(jpg_path).unwrap().to_rgb8();

        let dzp_bytes = dzp_converter.convert_image(&img);

        let mut dzp = File::create(&dzp_path).unwrap();
        dzp.write_all(&dzp_bytes).unwrap();

        let elapsed = now.elapsed();

        println!(
            "Created {} in {:.3}s",
            dzp_path.file_name().unwrap().to_string_lossy(),
            elapsed.as_secs_f64()
        );
    }
}
