//! Decode, transform and re-encode images for the `notia-image-editor` desktop app.
//!
//! The crate is pure Rust — png, jpeg and webp go through the `image` crate with no
//! native codec libraries — and has no GUI dependency, so it is the whole test
//! surface on CI.
//!
//! ```no_run
//! use edit::ImageDoc;
//!
//! # fn main() -> edit::Result<()> {
//! let mut doc = ImageDoc::open("photo.jpg")?; // EXIF orientation already applied
//! doc.rotate(1)?;                              // 90° clockwise
//! doc.save_as("photo-rotated.jpg")?;           // the source file is untouched
//! # Ok(())
//! # }
//! ```
//!
//! Design rules that the tests hold onto:
//!
//! - Opening applies the EXIF orientation, so a phone photo is upright immediately.
//! - Every mutating operation pushes an undo frame; at most [`MAX_UNDO`] frames are kept.
//! - Decoding refuses more than [`MAX_PIXELS`] pixels with [`EditError::TooManyPixels`]
//!   instead of letting the allocator die.
//! - An edit never overwrites the source. `save` needs a save target, which only
//!   `save_as` sets; until then it fails with [`EditError::NoSaveTarget`].

mod document;
mod error;

pub use document::ImageDoc;
pub use error::{EditError, Result};
pub use image::metadata::Orientation;

/// Ceiling for a decoded image, in pixels (80 MP ≈ 320 MB as RGBA8).
pub const MAX_PIXELS: u64 = 80_000_000;

/// Number of in-memory undo frames kept per document.
pub const MAX_UNDO: usize = 8;

/// Returns [`EditError::TooManyPixels`] when `width × height` exceeds [`MAX_PIXELS`].
///
/// Called on the decoder's declared dimensions before any pixel buffer is allocated,
/// and again on the result of a resize.
pub fn check_pixel_budget(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PIXELS {
        return Err(EditError::TooManyPixels {
            width,
            height,
            pixels,
            limit: MAX_PIXELS,
        });
    }
    Ok(())
}
