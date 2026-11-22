//! Tests for open/save behaviour: EXIF orientation, megapixel refusal, and the
//! "an edit never touches the opened file" contract.

use std::{
    fs,
    path::{Path, PathBuf},
};

use edit::{EditError, ImageDoc, MAX_PIXELS};
use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::jpeg::JpegEncoder};

const RED: Rgba<u8> = Rgba([220, 20, 20, 255]);
const GREEN: Rgba<u8> = Rgba([20, 200, 20, 255]);

/// Four coloured quadrants: red top-left, green top-right, blue bottom-left,
/// yellow bottom-right. Big blocks survive JPEG compression well enough to compare.
fn quadrants(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = match (x < width / 2, y < height / 2) {
            (true, true) => RED,
            (false, true) => GREEN,
            (true, false) => Rgba([20, 20, 220, 255]),
            (false, false) => Rgba([230, 220, 20, 255]),
        };
    }
    img
}

/// A JPEG whose EXIF orientation tag is `exif_orientation`, built by splicing an
/// APP1 segment right after the SOI marker of an otherwise normal JPEG.
fn jpeg_with_exif_orientation(img: &RgbaImage, exif_orientation: u16) -> Vec<u8> {
    let rgb: Vec<u8> = img
        .pixels()
        .flat_map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
        .collect();
    let mut plain = Vec::new();
    JpegEncoder::new_with_quality(&mut plain, 95)
        .write_image(&rgb, img.width(), img.height(), ExtendedColorType::Rgb8)
        .expect("encode fixture jpeg");

    // Exif payload: "Exif\0\0" + little-endian TIFF header + one IFD entry (tag 0x0112).
    let mut exif = Vec::new();
    exif.extend_from_slice(b"Exif\0\0");
    exif.extend_from_slice(b"II\x2a\x00"); // little-endian, magic 42
    exif.extend_from_slice(&8u32.to_le_bytes()); // IFD0 offset
    exif.extend_from_slice(&1u16.to_le_bytes()); // one entry
    exif.extend_from_slice(&0x0112u16.to_le_bytes()); // Orientation
    exif.extend_from_slice(&3u16.to_le_bytes()); // SHORT
    exif.extend_from_slice(&1u32.to_le_bytes()); // count
    exif.extend_from_slice(&exif_orientation.to_le_bytes());
    exif.extend_from_slice(&[0, 0]); // value padding
    exif.extend_from_slice(&0u32.to_le_bytes()); // no next IFD

    let segment_len = (exif.len() + 2) as u16;
    let mut out = Vec::with_capacity(plain.len() + exif.len() + 4);
    out.extend_from_slice(&plain[..2]); // SOI
    out.extend_from_slice(&[0xFF, 0xE1]); // APP1
    out.extend_from_slice(&segment_len.to_be_bytes());
    out.extend_from_slice(&exif);
    out.extend_from_slice(&plain[2..]);
    out
}

fn close_to(actual: Rgba<u8>, expected: Rgba<u8>) -> bool {
    actual
        .0
        .iter()
        .zip(expected.0.iter())
        .all(|(a, b)| a.abs_diff(*b) < 40)
}

#[test]
fn open_applies_exif_orientation_from_jpeg() {
    // Landscape 64x32 that a phone marked "rotate 90 CW to view" (orientation 6).
    let source = quadrants(64, 32);
    let bytes = jpeg_with_exif_orientation(&source, 6);

    let doc = ImageDoc::from_bytes(&bytes).expect("decode oriented jpeg");

    // Upright means portrait.
    assert_eq!((doc.width(), doc.height()), (32, 64));

    // Rotating 90° clockwise moves the source bottom-left pixel to the top-left.
    let upright = doc.pixels();
    assert!(
        close_to(*upright.get_pixel(4, 4), Rgba([20, 20, 220, 255])),
        "top-left should be the source bottom-left (blue), got {:?}",
        upright.get_pixel(4, 4)
    );
    // And the source top-left (red) ends up top-right.
    assert!(
        close_to(*upright.get_pixel(28, 4), RED),
        "top-right should be red, got {:?}",
        upright.get_pixel(28, 4)
    );
}

#[test]
fn open_ignores_a_normal_exif_orientation() {
    let source = quadrants(64, 32);
    let bytes = jpeg_with_exif_orientation(&source, 1);

    let doc = ImageDoc::from_bytes(&bytes).expect("decode plain jpeg");

    assert_eq!((doc.width(), doc.height()), (64, 32));
    assert!(close_to(*doc.pixels().get_pixel(4, 4), RED));
}

#[test]
fn oversize_image_is_refused_with_a_typed_error() {
    // 100_000 x 100_000 PNG header: the refusal has to happen on the declared
    // dimensions, before any pixel buffer exists.
    let width: u32 = 100_000;
    let height: u32 = 100_000;
    assert!(u64::from(width) * u64::from(height) > MAX_PIXELS);

    let bytes = png_with_declared_size(width, height);
    match ImageDoc::from_bytes(&bytes) {
        Err(EditError::TooManyPixels {
            width: w,
            height: h,
            limit,
            ..
        }) => {
            assert_eq!((w, h, limit), (width, height, MAX_PIXELS));
        }
        other => panic!("expected TooManyPixels, got {other:?}"),
    }
}

/// Builds a valid 1x1 PNG and rewrites its IHDR dimensions (with a fresh CRC), so
/// the file decodes nowhere near the size it declares.
fn png_with_declared_size(width: u32, height: u32) -> Vec<u8> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(RED.0.as_slice(), 1, 1, ExtendedColorType::Rgba8)
        .expect("encode fixture png");

    // PNG layout: 8-byte signature, then IHDR chunk: len(4) type(4) data(13) crc(4).
    const SIGNATURE_LEN: usize = 8;
    const CHUNK_HEADER_LEN: usize = 8;
    let data_start = SIGNATURE_LEN + CHUNK_HEADER_LEN;
    png[data_start..data_start + 4].copy_from_slice(&width.to_be_bytes());
    png[data_start + 4..data_start + 8].copy_from_slice(&height.to_be_bytes());
    let crc = crc32(&png[SIGNATURE_LEN + 4..data_start + 13]);
    let crc_start = data_start + 13;
    png[crc_start..crc_start + 4].copy_from_slice(&crc.to_be_bytes());
    png
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("notia-image-editor-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn write_png(path: &Path, img: &RgbaImage) {
    img.save_with_format(path, image::ImageFormat::Png)
        .expect("write fixture png");
}

#[test]
fn source_is_never_modified_and_save_as_writes_a_new_file() {
    let dir = scratch_dir("save");
    let source_path = dir.join("source.png");
    let target_path = dir.join("rotated.png");
    write_png(&source_path, &quadrants(64, 32));
    let original = fs::read(&source_path).expect("read source back");

    let mut doc = ImageDoc::open(&source_path).expect("open source png");
    doc.rotate_clockwise().expect("rotate");

    // No save target yet: save must refuse rather than guess the source.
    assert!(matches!(doc.save(), Err(EditError::NoSaveTarget)));
    assert_eq!(fs::read(&source_path).unwrap(), original);

    doc.save_as(&target_path).expect("save as");
    assert_eq!(
        fs::read(&source_path).unwrap(),
        original,
        "source must be untouched"
    );

    let saved = image::open(&target_path).expect("reopen saved file");
    assert_eq!((saved.width(), saved.height()), (32, 64));

    // Save now reuses the chosen target, still without touching the source.
    doc.rotate_clockwise().expect("rotate again");
    doc.save().expect("save to existing target");
    assert_eq!(fs::read(&source_path).unwrap(), original);
    let resaved = image::open(&target_path).expect("reopen resaved file");
    assert_eq!((resaved.width(), resaved.height()), (64, 32));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn save_to_unknown_extension_is_an_error() {
    let dir = scratch_dir("extension");
    let source_path = dir.join("source.png");
    write_png(&source_path, &quadrants(8, 8));

    let mut doc = ImageDoc::open(&source_path).expect("open");
    assert!(matches!(
        doc.save_as(dir.join("out.bmp")),
        Err(EditError::UnsupportedOutputFormat(_))
    ));
    assert!(matches!(
        doc.save_as(dir.join("out")),
        Err(EditError::UnknownOutputFormat(_))
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn jpeg_and_webp_round_trip() {
    let dir = scratch_dir("formats");
    let source_path = dir.join("source.png");
    write_png(&source_path, &quadrants(32, 16));
    let mut doc = ImageDoc::open(&source_path).expect("open");

    doc.save_as(dir.join("out.jpg")).expect("save jpeg");
    doc.save_as(dir.join("out.webp")).expect("save webp");

    for name in ["out.jpg", "out.webp"] {
        let reopened = ImageDoc::open(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((reopened.width(), reopened.height()), (32, 16), "{name}");
    }

    let _ = fs::remove_dir_all(&dir);
}
