mod dzi;

use image::{Rgb, RgbImage};
use std::f64::consts::PI;
use std::path::Path;
use dzi::TileCreator;
use rayon::prelude::*;
use image::imageops::{interpolate_bilinear, interpolate_nearest, tile};

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
            Face::Back => "back",
            Face::Down => "down",
            Face::Front => "front",
            Face::Left => "left",
            Face::Right => "right",
            Face::Up => "up",
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
    let src = image::open("/Users/george/Downloads/panos/recording_2024-07-30_04-51-55_00002.jpg").unwrap().to_rgb8();
    let face_size = src.width() / 4;
    faces.par_iter().for_each(|face| {
        let output_path = format!("/Users/george/Downloads/panos/100mpix_bilinear/{}.jpg", face.suffix());
        let result = render_face(&src, *face, face_size);
        result.save(&output_path).unwrap();

        // dzi it
        let tile_size = 512;
        let levels = (face_size as f64 / tile_size as f64).sqrt().ceil() as u32 + 1;
        let creator = TileCreator::new_from_image_path(Path::new(output_path.as_str()), 512, 0, Some(levels)).unwrap();
        creator.create_tiles().unwrap();
    });

    // now that we have all the temp folders, lets combine them into an uncompressed zip

    let elapsed = now.elapsed();
    println!("bilinear_kernel: {:.2?}", elapsed);

    println!("all done");
}
