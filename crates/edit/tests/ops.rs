//! Tests for the transform operations and the undo cap.

use edit::{EditError, ImageDoc, MAX_UNDO};
use image::{Rgba, RgbaImage};

fn solid(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_pixel(width, height, color)
}

/// Gradient image so rotation mistakes change pixels, not just dimensions.
fn gradient(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 3) as u8, (y * 5) as u8, (x + y) as u8, 255]);
    }
    img
}

fn doc_from(img: &RgbaImage) -> ImageDoc {
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .expect("encode fixture");
    ImageDoc::from_bytes(&bytes).expect("decode fixture")
}

#[test]
fn rotate_four_times_returns_the_original_pixels() {
    let original = gradient(40, 24);
    let mut doc = doc_from(&original);

    for _ in 0..4 {
        doc.rotate_clockwise().expect("rotate");
    }

    assert_eq!((doc.width(), doc.height()), (40, 24));
    assert_eq!(doc.pixels(), &original, "4 × 90° must be the identity");
}

#[test]
fn rotate_swaps_dimensions_and_moves_corners() {
    let mut img = solid(40, 24, Rgba([9, 9, 9, 255]));
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255])); // top-left marker

    let mut doc = doc_from(&img);
    doc.rotate(1).expect("rotate 90 cw");

    assert_eq!((doc.width(), doc.height()), (24, 40));
    // 90° clockwise sends top-left to top-right.
    assert_eq!(*doc.pixels().get_pixel(23, 0), Rgba([255, 0, 0, 255]));
}

#[test]
fn rotate_rejects_more_than_three_quarters() {
    let mut doc = doc_from(&gradient(8, 8));
    assert!(matches!(doc.rotate(4), Err(EditError::InvalidRotation(4))));
    assert_eq!(
        doc.undo_depth(),
        0,
        "a rejected rotation must not push undo"
    );
}

#[test]
fn resize_keeps_aspect_by_default() {
    let mut doc = doc_from(&gradient(40, 20));

    doc.resize(60, 5, true).expect("aspect-locked resize");

    assert_eq!(
        (doc.width(), doc.height()),
        (60, 30),
        "height follows width"
    );
}

#[test]
fn resize_without_aspect_uses_both_dimensions() {
    let mut doc = doc_from(&gradient(40, 20));

    doc.resize(60, 5, false).expect("free resize");

    assert_eq!((doc.width(), doc.height()), (60, 5));
}

#[test]
fn resize_rejects_zero_sides_and_oversize_results() {
    let mut doc = doc_from(&gradient(40, 20));

    assert!(matches!(
        doc.resize(0, 10, true),
        Err(EditError::InvalidResize { .. })
    ));
    assert!(matches!(
        doc.resize(40, 0, false),
        Err(EditError::InvalidResize { .. })
    ));
    assert!(matches!(
        doc.resize(20_000, 20_000, false),
        Err(EditError::TooManyPixels { .. })
    ));
    assert_eq!(doc.undo_depth(), 0);
}

#[test]
fn crop_keeps_the_requested_rectangle() {
    let mut doc = doc_from(&gradient(40, 24));

    doc.crop(8, 4, 10, 6).expect("in-bounds crop");

    assert_eq!((doc.width(), doc.height()), (10, 6));
    assert_eq!(*doc.pixels().get_pixel(0, 0), Rgba([24, 20, 12, 255]));
}

#[test]
fn crop_out_of_bounds_is_refused() {
    let mut doc = doc_from(&gradient(40, 24));

    for (x, y, w, h) in [
        (35, 0, 10, 10),
        (0, 20, 10, 10),
        (0, 0, 0, 10),
        (0, 0, 10, 0),
    ] {
        assert!(
            matches!(doc.crop(x, y, w, h), Err(EditError::InvalidCrop { .. })),
            "crop {w}x{h} at ({x}, {y}) should be refused"
        );
    }
    assert_eq!((doc.width(), doc.height()), (40, 24));
    assert_eq!(doc.undo_depth(), 0);
}

#[test]
fn undo_cap_keeps_the_newest_frames() {
    let mut doc = doc_from(&gradient(40, 24));

    // Each crop shrinks the width by one, so history is visible in the dimensions.
    for _ in 0..(MAX_UNDO + 4) {
        let (width, height) = (doc.width(), doc.height());
        doc.crop(0, 0, width - 1, height).expect("crop");
    }
    assert_eq!(
        (doc.width(), doc.height()),
        (40 - (MAX_UNDO + 4) as u32, 24)
    );
    assert_eq!(
        doc.undo_depth(),
        MAX_UNDO,
        "undo stack is capped at {MAX_UNDO}"
    );

    for _ in 0..MAX_UNDO {
        doc.undo().expect("undo");
    }
    assert!(!doc.can_undo());
    assert!(matches!(doc.undo(), Err(EditError::NothingToUndo)));
    assert_eq!(
        (doc.width(), doc.height()),
        (36, 24),
        "only the newest {MAX_UNDO} frames survive; the first four crops are gone"
    );
}

#[test]
fn undo_restores_the_frame_before_the_last_operation() {
    let original = gradient(40, 24);
    let mut doc = doc_from(&original);

    doc.crop(0, 0, 10, 10).expect("crop");
    assert_eq!((doc.width(), doc.height()), (10, 10));

    doc.undo().expect("undo crop");

    assert_eq!(doc.pixels(), &original);
}
