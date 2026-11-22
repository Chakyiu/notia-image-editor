# notia-image-editor

A desktop editor for crop, rotate and resize. This repository currently holds the
`edit` library — the pixel work — with no window yet; the GUI lands in a follow-up.

## `edit`

Pure Rust, no GUI dependency, no native codec libraries. png, jpeg and webp are
decoded and encoded through the [`image`](https://crates.io/crates/image) crate, so
CI on Windows needs no libpng, vcpkg or ImageMagick.

What it does:

- **Open** a png/jpeg/webp and apply its EXIF orientation, so a phone photo is
  upright before anyone touches a toolbar.
- **Rotate** in 90° steps.
- **Resize** by pixel width and height, with the aspect ratio locked by default.
- **Crop** to a rectangle that must fit inside the image.
- **Undo** in memory, capped at 8 frames, so a bad crop is recoverable.
- **Save / Save As**, never overwriting the opened file as a side effect of an edit:
  `save` fails until `save_as` has picked a destination.
- **Refuse** a decode above 80 megapixels with `EditError::TooManyPixels` instead of
  dying in the allocator (RGBA at 50 MP is already ~200 MB, and undo multiplies it).

```rust
use edit::ImageDoc;

fn main() -> edit::Result<()> {
    let mut doc = ImageDoc::open("photo.jpg")?; // EXIF-applied, RGBA8
    doc.rotate(1)?;                              // 90° clockwise
    doc.resize(1280, 0, true)?;                  // height follows the width
    doc.save_as("photo-small.jpg")?;             // photo.jpg is untouched
    Ok(())
}
```

Saving flattens alpha for JPEG, which has no alpha channel; png and webp keep it.

## Layout

```
Cargo.toml          workspace (edition 2024, Rust 1.88, MIT)
crates/edit/        the library and its tests
.github/workflows/  fmt + clippy + test on ubuntu, macos, windows
```

## Tests

```sh
cargo test --workspace
```

The suite covers EXIF orientation from a JPEG built in the test, rotate × 4 returning
the original pixels, aspect-locked resize, in-bounds and out-of-bounds crop, the
megapixel refusal (a PNG whose header declares 100 000 × 100 000), the 8-frame undo
cap, and that `save_as` writes a new file while leaving the source byte-identical.

Lint and format the same way CI does:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

## License

MIT.
