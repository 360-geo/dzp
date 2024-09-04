use std::collections::HashMap;
use image::{DynamicImage, GenericImageView, ImageError};
use image::codecs::jpeg::JpegEncoder;

#[derive(thiserror::Error, Debug)]
pub enum TilingError {
    #[error("Unsupported source image: {0}")]
    UnsupportedSourceImage(String),
    #[error("Unexpected error")]
    UnexpectedError,
    #[error("Unsupported source image: {0}")]
    ImageError(#[from] ImageError),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

pub type DZIResult<T, E = TilingError> = Result<T, E>;

/// A tile creator, this struct and associated functions
/// implement the DZI tiler
pub struct TileCreator {
    /// source image
    pub image: DynamicImage,
    /// source image
    pub name: String,
    /// size of individual tiles in pixels
    pub tile_size: u32,
    /// number of pixels neighboring tiles overlap
    pub tile_overlap: u32,
    /// total number of levels of tiles
    pub levels: u32,
    /// the buffer
    buffer: HashMap<String, Vec<u8>>,
}

impl TileCreator {
    pub fn new_from_image(im: DynamicImage, name: String, tile_size: u32, tile_overlap: u32, levels: Option<u32>) -> DZIResult<Self> {
        // let im = image::io::Reader::open(image_path)?
        //     .with_guessed_format()?
        //     .decode()?;
        let (h, w) = im.dimensions();

        let actual_levels = if levels.is_none() {
            (h.max(w) as f64).log2().ceil() as u32 + 1
        } else {
            levels.unwrap()
        };

        Ok(Self {
            image: im,
            name,
            tile_size,
            tile_overlap,
            levels: actual_levels,
            buffer: HashMap::new(),
        })
    }

    /// Create DZI tiles
    pub fn create_tiles(mut self) -> DZIResult<HashMap<String, Vec<u8>>> {
        for l in 0..self.levels {
            self.create_level(l)?;
        }

        let (w, h) = self.image.dimensions();

        let dzi = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Image xmlns="http://schemas.microsoft.com/deepzoom/2008"
    TileSize="{}"
    Overlap="{}"
    Format="jpg">
    <Size Width="{}" Height="{}"/>
</Image>"#,
          self.tile_size,
          self.tile_overlap,
          w,
          h
        );

        self.buffer.insert(format!("{}.dzi", self.name), dzi.as_bytes().to_vec());

        Ok(self.buffer.to_owned())
    }

    /// Check if level is valid
    fn check_level(&self, l: u32) -> DZIResult<()> {
        if l >= self.levels {
            return Err(TilingError::UnexpectedError);
        }
        Ok(())
    }

    /// Create tiles for a level
    fn create_level(&mut self, level: u32) -> DZIResult<()> {
        let mut li = self.get_level_image(level)?;
        let (c, r) = self.get_tile_count(level)?;
        for col in 0..c {
            for row in 0..r {
                let (x, y, x2, y2) = self.get_tile_bounds(level, col, row)?;
                let tile_image = li.crop(x, y, x2 - x, y2 - y);
                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, 90);
                tile_image.write_with_encoder(encoder)?;
                self.buffer.insert(format!("{}_files/{}/{}_{}.jpg", self.name, level, col, row), buffer);
            }
        }
        Ok(())
    }

    /// Get image for a level
    fn get_level_image(&self, level: u32) -> DZIResult<DynamicImage> {
        self.check_level(level)?;

        let (w, h) = self.get_dimensions(level)?;

        Ok(self.image
            .resize(
                w,
                h,
                image::imageops::FilterType::Lanczos3,
            )
        )
    }

    /// Get scale factor at level
    fn get_scale(&self, level: u32) -> DZIResult<f64> {
        self.check_level(level)?;
        Ok(0.5f64.powi((self.levels - 1 - level) as i32))
    }

    /// Get dimensions (width, height) in pixels of image for level
    fn get_dimensions(&self, level: u32) -> DZIResult<(u32, u32)> {
        self.check_level(level)?;
        let s = self.get_scale(level)?;
        let (w, h) = self.image.dimensions();
        let h = (h as f64 * s).ceil() as u32;
        let w = (w as f64 * s).ceil() as u32;
        Ok((w, h))
    }

    /// Get (number of columns, number of rows) for a level
    fn get_tile_count(&self, l: u32) -> DZIResult<(u32, u32)> {
        let (w, h) = self.get_dimensions(l)?;
        let cols = (w as f64 / self.tile_size as f64).ceil() as u32;
        let rows = (h as f64 / self.tile_size as f64).ceil() as u32;
        Ok((cols, rows))
    }

    fn get_tile_bounds(
        &self,
        level: u32,
        col: u32,
        row: u32,
    ) -> DZIResult<(u32, u32, u32, u32)> {
        let offset_x = if col == 0 {
            0
        } else {
            self.tile_overlap
        };
        let offset_y = if row == 0 {
            0
        } else {
            self.tile_overlap
        };
        let x = col * self.tile_size - offset_x;
        let y = row * self.tile_size - offset_y;

        let (lw, lh) = self.get_dimensions(level)?;

        let w = self.tile_size +
            (if col == 0 { 1 } else { 2 }) * self.tile_overlap;
        let h = self.tile_size +
            (if row == 0 { 1 } else { 2 }) * self.tile_overlap;

        let w = w.min(lw - x);
        let h = h.min(lh - y);
        Ok((x, y, x + w, y + h))
    }
}
