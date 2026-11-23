//! Native Notia plugin (optional `notia` feature) and helpers shared with the JS plugin.

mod embed;
pub mod image_path;

pub use embed::{decode_base64, encode_base64};
pub use image_path::{image_target_from_line, is_supported_image_path};

#[cfg(feature = "notia")]
mod native;

#[cfg(feature = "notia")]
pub use native::ImageEditorPlugin;