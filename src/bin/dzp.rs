use std::collections::HashMap;
use dzp::dzi::{TileCreator};
use image::{DynamicImage, GenericImage};
use std::f64::consts::PI;
use std::fs::File;
use std::io::prelude::*;
use rayon::prelude::*;
use image::imageops::{interpolate_bilinear, interpolate_nearest};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};
use dzp::Face;

fn mod_2pi(x: f64) -> f64 {
    ((x % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI)
}

fn render_face(
    src: &DynamicImage,
    max_width: u32,
    pixel_coordinate_cache: &PixelCoordinateCache,
) -> DynamicImage {
    let face_width = max_width.min(src.width() / 4);
    let face_height = face_width;

    let mut dst = DynamicImage::new(face_width, face_height, src.color());

    for pixel_mapping in pixel_coordinate_cache {
        let colour = interpolate_bilinear(
            src,
            pixel_mapping.source_coordinate.0,
            pixel_mapping.source_coordinate.1,
        );

        dst.put_pixel(pixel_mapping.face_coordinate.0, pixel_mapping.face_coordinate.1, colour.unwrap());
    }

    dst
}

#[derive(Debug)]
struct PixelMapping {
    source_coordinate: (f32, f32),
    face_coordinate: (u32, u32),
}
type PixelCoordinateCache = Vec<PixelMapping>;
type FaceCache = HashMap<Face, PixelCoordinateCache>;
type FaceSizeCache = HashMap<(u32, u32), FaceCache>;

fn generate_cache_for_resolution(resolution: (u32, u32), face_size_cache: &mut FaceSizeCache) {
    let faces = [
        Face::Front,
        Face::Back,
        Face::Left,
        Face::Right,
        Face::Down,
        Face::Up,
    ];

    let face_size = resolution.0 / 4;

    let mut face_cache: FaceCache = HashMap::new();

    for face in faces {
        let orientation = match face {
            Face::Front => |x: f64, y: f64| (-1.0, -x, -y),
            Face::Back => |x: f64, y: f64| (1.0, x, -y),
            Face::Left => |x: f64, y: f64| (-x, 1.0, -y),
            Face::Right => |x: f64, y: f64| (x, -1.0, -y),
            Face::Down => |x: f64, y: f64| (y, -x, -1.0),
            Face::Up => |x: f64, y: f64| (-y, -x, 1.0),
        };

        let mut pixel_coordinate_cache: PixelCoordinateCache = Vec::new();

        for x in 0..face_size {
            for y in 0..face_size {
                let (cube_x, cube_y, cube_z) = orientation(
                    2.0 * (x as f64 + 0.5) / face_size as f64 - 1.0,
                    2.0 * (y as f64 + 0.5) / face_size as f64 - 1.0,
                );

                let r = (cube_x * cube_x + cube_y * cube_y + cube_z * cube_z).sqrt();
                let lon = mod_2pi(cube_y.atan2(cube_x));
                let lat = (cube_z / r).acos();

                let src_x = resolution.0 as f64 * lon / (2.0 * PI) - 0.5;
                let src_y = resolution.1 as f64 * lat / PI - 0.5;

                let mapping = PixelMapping {
                    source_coordinate: (src_x as f32, src_y as f32),
                    face_coordinate: (x, y)
                };

                pixel_coordinate_cache.push(mapping);
            }
        }
        face_cache.insert(face, pixel_coordinate_cache);
    }

    face_size_cache.insert(resolution, face_cache);
}

pub fn main() {
    let faces =[
        Face::Front,
        Face::Back,
        Face::Left,
        Face::Right,
        Face::Down,
        Face::Up,
    ];

    use std::time::Instant;

    let mut face_size_cache: FaceSizeCache = HashMap::new();

    let files = [
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00030.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00030.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00031.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00031.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00032.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00032.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00033.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00033.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00034.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00034.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00035.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00035.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00036.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00036.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00037.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00037.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00038.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00038.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00039.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00039.dzp"),
        ("/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00040.jpg", "/Users/george/Downloads/dzp_test/recording_2024-06-28_05-32-06_00040.dzp"),
    ];

    // ensure cache is warm for testing
    generate_cache_for_resolution((14400, 7200), &mut face_size_cache);

    for (source, target) in files {
        let now = Instant::now();
        let src = image::open(source).unwrap();

        let resolution = (src.width(), src.height());

        if ! face_size_cache.contains_key(&resolution) {
            generate_cache_for_resolution(resolution, &mut face_size_cache);
        }

        let face_cache = face_size_cache.get(&resolution).unwrap();

        let face_size = src.width() / 4;
        let file_systems = faces.clone().par_iter().map(|face| {
            let pixel_coordinate_cache = face_cache.get(&face).unwrap();
            let result = render_face(&src, face_size, pixel_coordinate_cache);

            // dzi it
            let tile_size = 512;
            let levels = (face_size as f64 / tile_size as f64).sqrt().ceil() as u32 + 1;
            let creator = TileCreator::new_from_image(result, face.suffix().to_string(), 512, 0, Some(levels)).unwrap();

            creator.create_tiles().unwrap()
        }).collect::<Vec<HashMap<String, Vec<u8>>>>();

        let dzp = File::create(target).unwrap();
        let mut dzp_writer = ZipWriter::new(dzp);
        let dzp_writer_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);

        for fs in file_systems {
            for (path, bytes) in fs {
                dzp_writer.start_file(path, dzp_writer_options).unwrap();
                dzp_writer.write_all(&bytes).unwrap();
            }
        }

        let elapsed = now.elapsed();
        println!("done in: {:.2?}", elapsed);
    }

    println!("all done");
}
