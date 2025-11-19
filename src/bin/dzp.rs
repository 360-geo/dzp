use clap::Parser;
use dzp::DzpConverter;
use image::ImageReader;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;
use tracing_subscriber::EnvFilter;

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

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stdout)
        .init();

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

        let image = ImageReader::open(&jpg_path)
            .unwrap()
            .decode()
            .unwrap()
            .into_rgb8();

        let encoding_time = now.elapsed();

        let dzp_bytes = dzp_converter.convert_image(&image);

        let mut dzp = File::create(&dzp_path).unwrap();
        dzp.write_all(&dzp_bytes).unwrap();

        let elapsed = now.elapsed();

        info!(
            "Created {} in {:.3}s, jpeg decoding took {:.3}s",
            dzp_path.file_name().unwrap().to_string_lossy(),
            elapsed.as_secs_f64(),
            encoding_time.as_secs_f64(),
        );
    }
}
