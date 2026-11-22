use std::{fmt, io, path::PathBuf};

/// Result alias for the crate.
pub type Result<T> = std::result::Result<T, EditError>;

/// Everything that can go wrong in the library.
#[derive(Debug)]
pub enum EditError {
    /// Reading or writing a file failed.
    Io(io::Error),
    /// The bytes are not a png/jpeg/webp image, or are corrupt.
    Decode(image::ImageError),
    /// The image cannot be encoded to the requested output format.
    Encode(image::ImageError),
    /// The output path has no extension the writer knows.
    UnknownOutputFormat(PathBuf),
    /// The output extension maps to a format this crate does not write.
    UnsupportedOutputFormat(PathBuf),
    /// Decoding the declared dimensions would exceed [`crate::MAX_PIXELS`].
    TooManyPixels {
        width: u32,
        height: u32,
        pixels: u64,
        limit: u64,
    },
    /// `rotate` only accepts 0, 1, 2 or 3 quarter turns.
    InvalidRotation(u8),
    /// A resize with a zero side, or an aspect-locked resize that rounds to zero.
    InvalidResize { width: u32, height: u32 },
    /// The crop rectangle is empty or not fully inside the image.
    InvalidCrop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        image_width: u32,
        image_height: u32,
    },
    /// `undo` was called with an empty undo stack.
    NothingToUndo,
    /// `save` was called before `save_as` picked a destination.
    NoSaveTarget,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "file i/o failed: {err}"),
            Self::Decode(err) => write!(f, "could not decode image: {err}"),
            Self::Encode(err) => write!(f, "could not encode image: {err}"),
            Self::UnknownOutputFormat(path) => {
                write!(
                    f,
                    "save path has no known image extension: {}",
                    path.display()
                )
            }
            Self::UnsupportedOutputFormat(path) => {
                write!(
                    f,
                    "save path extension is not written by this crate: {}",
                    path.display()
                )
            }
            Self::TooManyPixels {
                width,
                height,
                pixels,
                limit,
            } => write!(
                f,
                "image is too large: {width}x{height} is {pixels} pixels, limit is {limit}"
            ),
            Self::InvalidRotation(turns) => {
                write!(f, "rotation must be 0-3 quarter turns, got {turns}")
            }
            Self::InvalidResize { width, height } => {
                write!(f, "resize to {width}x{height} is not allowed")
            }
            Self::InvalidCrop {
                x,
                y,
                width,
                height,
                image_width,
                image_height,
            } => write!(
                f,
                "crop {width}x{height} at ({x}, {y}) does not fit inside {image_width}x{image_height}"
            ),
            Self::NothingToUndo => write!(f, "nothing to undo"),
            Self::NoSaveTarget => write!(f, "no save target: use save_as to pick a file first"),
        }
    }
}

impl std::error::Error for EditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Decode(err) | Self::Encode(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for EditError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
