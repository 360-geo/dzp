use std::collections::HashMap;
use std::f64::consts::PI;
use std::io::Cursor;
use crate::dzi::TileCreator;
use image::imageops::interpolate_bilinear as interpolate_fn;
use rayon::prelude::*;
use std::io::prelude::*;
use std::sync::{Arc, RwLock};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

mod dzi;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Face {
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

    fn orientation(&self) -> fn(f64, f64) -> (f64, f64, f64) {
        match *self {
            Face::Front => |x: f64, y: f64| (-1.0, -x, -y),
            Face::Back => |x: f64, y: f64| (1.0, x, -y),
            Face::Left => |x: f64, y: f64| (-x, 1.0, -y),
            Face::Right => |x: f64, y: f64| (x, -1.0, -y),
            Face::Down => |x: f64, y: f64| (y, -x, -1.0),
            Face::Up => |x: f64, y: f64| (-y, -x, 1.0),
        }
    }
}

#[derive(Debug)]
struct PixelMapping {
    source_coordinate: (f32, f32),
    face_coordinate: (u32, u32),
}
type PixelCoordinateCache = Vec<PixelMapping>;
type FaceCache = HashMap<Face, PixelCoordinateCache>;
type FaceSizeCache = Arc<RwLock<HashMap<u32, FaceCache>>>;

pub struct DzpConverter {
    face_size_cache: FaceSizeCache,
    faces: [Face; 6],
}

impl DzpConverter {
    /// Create an instance of the converter
    pub fn create() -> Self {
        Self {
            face_size_cache: Arc::new(RwLock::new(HashMap::new())),
            faces: [
                Face::Front,
                Face::Back,
                Face::Left,
                Face::Right,
                Face::Down,
                Face::Up,
            ],
        }
    }

    /// Convert an image into a dzp, and return the bytes of the container as a vector
    pub fn convert_image(&self, image: &image::RgbImage) -> Vec<u8> {
        let resolution = (image.width(), image.height());
        assert_eq!(resolution.0, resolution.1 * 2);

        let should_generate_cache;
        {
            let cache = self.face_size_cache.read().unwrap();
            should_generate_cache = !cache.contains_key(&resolution.0);
        }

        if should_generate_cache {
            self.generate_cache_for_resolution(resolution.0);
        }

        let file_systems;
        {
            let cache = self.face_size_cache.read().unwrap();
            let face_cache = cache.get(&resolution.0).unwrap();

            let face_size = image.width() / 4;
            file_systems = self
                .faces
                .clone()
                .par_iter()
                .map(|face| {
                    let pixel_coordinate_cache = face_cache.get(face).unwrap();
                    let result = self.render_face(image, face_size, pixel_coordinate_cache);

                    // dzi it
                    let tile_size = 512;
                    let levels = (face_size as f64 / tile_size as f64).sqrt().ceil() as u32 + 1;
                    let creator = TileCreator::new_from_image(
                        result,
                        face.suffix().to_string(),
                        512,
                        0,
                        Some(levels),
                    )
                    .unwrap();

                    creator.create_tiles().unwrap()
                })
                .collect::<Vec<HashMap<String, Vec<u8>>>>();
        }

        let mut buffer = Cursor::new(Vec::new());

        {
            let mut dzp_writer = ZipWriter::new(&mut buffer);
            let dzp_writer_options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o755);

            for fs in file_systems {
                for (path, bytes) in fs {
                    dzp_writer.start_file(path, dzp_writer_options).unwrap();
                    dzp_writer.write_all(&bytes).unwrap();
                }
            }
        }

        buffer.into_inner()
    }

    fn render_face(
        &self,
        src: &image::RgbImage,
        max_width: u32,
        pixel_coordinate_cache: &PixelCoordinateCache,
    ) -> image::RgbImage {
        let face_width = max_width.min(src.width() / 4);
        let face_height = face_width;

        let mut dst = image::RgbImage::new(face_width, face_height);

        for pixel_mapping in pixel_coordinate_cache {
            let colour = interpolate_fn(
                src,
                pixel_mapping.source_coordinate.0,
                pixel_mapping.source_coordinate.1,
            );

            dst.put_pixel(
                pixel_mapping.face_coordinate.0,
                pixel_mapping.face_coordinate.1,
                colour.unwrap(),
            );
        }

        dst
    }

    fn generate_cache_for_resolution(&self, width: u32) {
        let height = width / 2;
        let face_size = width / 4;

        let mut face_cache: FaceCache = HashMap::new();

        for face in self.faces {
            let orientation = face.orientation();

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

                    let src_x = width as f64 * lon / (2.0 * PI) - 0.5;
                    let src_y = height as f64 * lat / PI - 0.5;

                    let mapping = PixelMapping {
                        source_coordinate: (src_x as f32, src_y as f32),
                        face_coordinate: (x, y),
                    };

                    pixel_coordinate_cache.push(mapping);
                }
            }
            face_cache.insert(face, pixel_coordinate_cache);
        }

        {
            let mut cache = self.face_size_cache.write().unwrap();
            cache.insert(width, face_cache);
        }
    }
}

fn mod_2pi(x: f64) -> f64 {
    ((x % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI)
}
