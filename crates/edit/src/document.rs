use std::{
    collections::VecDeque,
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageFormat, ImageReader, RgbaImage};

use crate::{EditError, MAX_UNDO, Result, check_pixel_budget};

/// One decoded image plus its edit history.
///
/// The pixels are always RGBA8 and always EXIF-upright: the orientation is applied
/// once, at open, and never again.
pub struct ImageDoc {
    pixels: RgbaImage,
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    undo: VecDeque<RgbaImage>,
}

impl ImageDoc {
    /// Opens a png/jpeg/webp file and applies its EXIF orientation.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader = ImageReader::open(path)?.with_guessed_format()?;
        Ok(Self {
            pixels: decode(reader)?,
            source: Some(path.to_path_buf()),
            target: None,
            undo: VecDeque::new(),
        })
    }

    /// Decodes an in-memory png/jpeg/webp and applies its EXIF orientation.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
        Ok(Self {
            pixels: decode(reader)?,
            source: None,
            target: None,
            undo: VecDeque::new(),
        })
    }

    /// Current pixels, RGBA8.
    pub fn pixels(&self) -> &RgbaImage {
        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    pub fn height(&self) -> u32 {
        self.pixels.height()
    }

    /// The file this document was opened from, if any.
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    /// The file `save` writes to; `None` until `save_as` has been called.
    pub fn target(&self) -> Option<&Path> {
        self.target.as_deref()
    }

    /// Number of undo frames currently held (at most [`MAX_UNDO`]).
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Rotates clockwise by `quarter_turns` × 90°.
    pub fn rotate(&mut self, quarter_turns: u8) -> Result<()> {
        if quarter_turns > 3 {
            return Err(EditError::InvalidRotation(quarter_turns));
        }
        if quarter_turns == 0 {
            return Ok(());
        }
        self.push_undo();
        self.pixels = match quarter_turns {
            1 => image::imageops::rotate90(&self.pixels),
            2 => image::imageops::rotate180(&self.pixels),
            _ => image::imageops::rotate270(&self.pixels),
        };
        Ok(())
    }

    /// Rotates 90° clockwise. Shorthand for `rotate(1)`.
    pub fn rotate_clockwise(&mut self) -> Result<()> {
        self.rotate(1)
    }

    /// Resizes to `width × height` pixels.
    ///
    /// With `keep_aspect` the height is derived from `width` so the aspect ratio is
    /// preserved; the `height` argument is then ignored.
    pub fn resize(&mut self, width: u32, height: u32, keep_aspect: bool) -> Result<()> {
        if width == 0 || (!keep_aspect && height == 0) {
            return Err(EditError::InvalidResize { width, height });
        }
        let (current_width, current_height) = (self.width(), self.height());
        let (width, height) = if keep_aspect {
            let height = u64::from(width)
                .saturating_mul(u64::from(current_height))
                .div_ceil(u64::from(current_width))
                .max(1)
                .min(u64::from(u32::MAX)) as u32;
            (width, height)
        } else {
            (width, height)
        };
        check_pixel_budget(width, height)?;
        if (width, height) == (current_width, current_height) {
            return Ok(());
        }
        self.push_undo();
        self.pixels = image::imageops::resize(
            &self.pixels,
            width,
            height,
            image::imageops::FilterType::Lanczos3,
        );
        Ok(())
    }

    /// Crops to the rectangle at (`x`, `y`) with the given size, in pixels.
    ///
    /// The rectangle must be non-empty and fully inside the image.
    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<()> {
        let (image_width, image_height) = (self.width(), self.height());
        let fits = width > 0
            && height > 0
            && u64::from(x) + u64::from(width) <= u64::from(image_width)
            && u64::from(y) + u64::from(height) <= u64::from(image_height);
        if !fits {
            return Err(EditError::InvalidCrop {
                x,
                y,
                width,
                height,
                image_width,
                image_height,
            });
        }
        self.push_undo();
        self.pixels = image::imageops::crop_imm(&self.pixels, x, y, width, height).to_image();
        Ok(())
    }

    /// Restores the pixels from before the most recent operation.
    pub fn undo(&mut self) -> Result<()> {
        let previous = self.undo.pop_back().ok_or(EditError::NothingToUndo)?;
        self.pixels = previous;
        Ok(())
    }

    /// Writes the pixels to the current save target.
    ///
    /// Fails with [`EditError::NoSaveTarget`] until [`Self::save_as`] has chosen a
    /// destination, which is what keeps an edit from overwriting the opened file.
    pub fn save(&mut self) -> Result<&Path> {
        let target = self.target.clone().ok_or(EditError::NoSaveTarget)?;
        self.write(&target)?;
        Ok(self.target.as_deref().expect("target was checked above"))
    }

    /// Writes the pixels to `path` and makes it the save target.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        self.write(path)?;
        self.target = Some(path.to_path_buf());
        Ok(())
    }

    fn push_undo(&mut self) {
        if self.undo.len() == MAX_UNDO {
            self.undo.pop_front();
        }
        self.undo.push_back(self.pixels.clone());
    }

    fn write(&self, path: &Path) -> Result<()> {
        let format = ImageFormat::from_path(path)
            .map_err(|_| EditError::UnknownOutputFormat(path.to_path_buf()))?;
        // JPEG has no alpha channel; flatten to RGB8 and drop it.
        let rgb;
        let (buffer, color) = if format == ImageFormat::Jpeg {
            rgb = self
                .pixels
                .pixels()
                .flat_map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
                .collect::<Vec<u8>>();
            (rgb.as_slice(), ExtendedColorType::Rgb8)
        } else if matches!(format, ImageFormat::Png | ImageFormat::WebP) {
            (self.pixels.as_raw().as_slice(), ExtendedColorType::Rgba8)
        } else {
            return Err(EditError::UnsupportedOutputFormat(path.to_path_buf()));
        };
        image::save_buffer_with_format(path, buffer, self.width(), self.height(), color, format)
            .map_err(EditError::Encode)
    }
}

/// Decodes to RGBA8, refusing oversized images before allocating their pixels.
fn decode<R: std::io::BufRead + std::io::Seek>(reader: ImageReader<R>) -> Result<RgbaImage> {
    let mut decoder = reader.into_decoder().map_err(EditError::Decode)?;
    let (width, height) = decoder.dimensions();
    check_pixel_budget(width, height)?;
    // `image::open` ignores this; a phone photo would otherwise open sideways.
    let orientation = decoder.orientation().map_err(EditError::Decode)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(EditError::Decode)?;
    image.apply_orientation(orientation);
    check_pixel_budget(image.width(), image.height())?;
    Ok(image.into_rgba8())
}

impl std::fmt::Debug for ImageDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageDoc")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("source", &self.source)
            .field("target", &self.target)
            .field("undo_depth", &self.undo.len())
            .finish()
    }
}
