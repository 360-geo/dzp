mod dzi;

use image::{RgbImage};
use std::f64::consts::PI;
use std::fmt::format;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use dzi::TileCreator;
use rayon::prelude::*;
use image::imageops::{interpolate_bilinear, interpolate_nearest};
use mktemp::Temp;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn mod_2pi(x: f64) -> f64 {
    ((x % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI)
}

#[derive(Copy, Clone, Debug)]
enum Face {
    Back,
    Down,
    Front,
    Left,
    Right,
    Up,
}

impl Face {
    fn suffix(&self) -> &'static str {
        match *self {
            Face::Back => "b",
            Face::Down => "d",
            Face::Front => "f",
            Face::Left => "l",
            Face::Right => "r",
            Face::Up => "u",
        }
    }
}

fn render_face(
    src: &RgbImage,
    face: Face,
    max_width: u32,
) -> RgbImage {
    let face_width = max_width.min(src.width() / 4);
    let face_height = face_width;

    let mut dst = RgbImage::new(face_width, face_height);

    let orientation = match face {
        Face::Front => |x: f64, y: f64| (-1.0, -x, -y),
        Face::Back => |x: f64, y: f64| (1.0, x, -y),
        Face::Left => |x: f64, y: f64| (-x, 1.0, -y),
        Face::Right => |x: f64, y: f64| (x, -1.0, -y),
        Face::Down => |x: f64, y: f64| (y, -x, -1.0),
        Face::Up => |x: f64, y: f64| (-y, -x, 1.0),
    };

    dst.enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, pixel)| {
            let (cube_x, cube_y, cube_z) = orientation(
                2.0 * (x as f64 + 0.5) / face_width as f64 - 1.0,
                2.0 * (y as f64 + 0.5) / face_height as f64 - 1.0,
            );

            let r = (cube_x * cube_x + cube_y * cube_y + cube_z * cube_z).sqrt();
            let lon = mod_2pi(cube_y.atan2(cube_x));
            let lat = (cube_z / r).acos();

            let src_x = src.width() as f64 * lon / (2.0 * PI) - 0.5;
            let src_y = src.height() as f64 * lat / PI - 0.5;

            *pixel = interpolate_bilinear(
                src,
                src_x as f32,
                src_y as f32,
            ).unwrap()
            // *pixel = interpolate_nearest(
            //     src,
            //     src_x as f32,
            //     src_y as f32,
            // ).unwrap()
        });

    dst
}

fn main() {
    let faces = vec![
        Face::Front,
        Face::Back,
        Face::Left,
        Face::Right,
        Face::Down,
        Face::Up,
    ];

    use std::time::Instant;

    let now = Instant::now();

    let src = image::open("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00030.jpg").unwrap().to_rgb8();
    let temp_path = Temp::new_dir().unwrap();
    let face_size = src.width() / 4;
    faces.clone().par_iter().for_each(|face| {
        let output_path = format!("{}/{}.jpg", temp_path.clone().to_str().unwrap(), face.suffix());
        let result = render_face(&src, *face, face_size);
        result.save(&output_path).unwrap();

        // dzi it
        let tile_size = 512;
        let levels = (face_size as f64 / tile_size as f64).sqrt().ceil() as u32 + 1;
        let creator = TileCreator::new_from_image_path(Path::new(output_path.as_str()), 512, 0, Some(levels)).unwrap();
        creator.create_tiles().unwrap();
    });

    let dzp = File::create("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00030.dzp").unwrap();
    let mut dzp_writer = ZipWriter::new(dzp);
    let dzp_writer_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o755);
    let mut buffer = Vec::new();

    // we need to add the generated files (example: `b_files/0/0_0.jpg`) to the zip file

    for face in faces {
        let face_dir_name = format!("{}_files", face.suffix());
        let face_dir = temp_path.clone().join(&face_dir_name);

        for entry in std::fs::read_dir(&face_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let level = path.file_name().unwrap();
                let level_dir = face_dir.join(&level);
                for entry in std::fs::read_dir(level_dir).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.extension().unwrap() == "jpg" {
                        // we have jpg
                        let tile = format!("{}/{}/{}", &face_dir_name, level.to_str().unwrap(), path.file_name().unwrap().to_str().unwrap());

                        dzp_writer.start_file(&tile, dzp_writer_options).unwrap();
                        let mut f = File::open(path).unwrap();
                        f.read_to_end(&mut buffer).unwrap();
                        dzp_writer.write_all(&buffer).unwrap();
                        buffer.clear();
                        println!("added {} to the dzp", tile)
                    }
                }

            }
        }
    }
    // now that we have all the temp folders, lets combine them into an uncompressed zip

    let elapsed = now.elapsed();
    println!("bilinear_kernel: {:.2?}", elapsed);

    println!("all done");
}
