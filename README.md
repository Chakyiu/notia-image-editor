# notia-image-editor

Crop and resize images embedded in [Notia](https://github.com/Chakyiu/notia) notes. The repo ships:

- **`edit`** — pure Rust decode/transform/encode (png, jpeg, webp)
- **`notia-image-editor` CLI** — apply crop/resize on disk
- **`manifest.json` / `main.js`** — community plugin (JavaScript) for **Settings → Plugins**
- **`notia-image-editor-plugin`** — optional native Notia plugin (writes vault image files)

MVP edit ops: **resize** (1280px width, aspect locked) and **crop** (10% margin trim).

## Install in Notia (community plugin)

1. Clone this repository.
2. Link it into your vault (replace paths):

```powershell
# PowerShell example
New-Item -ItemType Junction -Path "$env:USERPROFILE\Notes\.notia\plugins\notia-image-editor" -Target "C:\path\to\notia-image-editor"
```

On macOS/Linux, symlink the **`plugin`** folder (the folder that contains `manifest.json`), not the repo root:

```sh
ln -s ~/code/notia-image-editor ~/Notes/.notia/plugins/notia-image-editor
```

3. In Notia: **Settings → Plugins** → enable **Image Editor**.
4. Open the **Image Editor** sidebar panel, place the cursor on a line like `![](attachments/photo.png)`, then **Resize** or **Crop**.

### Native companion (writes vault files)

Community plugins cannot read or write binary vault attachments yet. The native crate registers the same commands and performs edits in-process:

1. From this repository (with a [Notia](https://github.com/Chakyiu/notia) checkout beside it or at `workdir/notia`):

```sh
cargo build -p notia-image-editor-plugin --features notia
```

2. Add the plugin to Notia’s `notia-plugins` crate (path dependency), register `ImageEditorPlugin` in `notia-plugins/src/lib.rs`, rebuild Notia, and enable **Image Editor** under **Settings → Plugins** (built-in list) *or* keep the community folder symlink for the panel UI.

Palette commands (native): **Image Editor: Resize embedded image at cursor (width 1280px)** and **Image Editor: Crop embedded image at cursor (10% margin)**.

## CLI

After `cargo build -p edit-cli --release`:

```sh
notia-image-editor info photo.png
notia-image-editor resize photo.png 1280
notia-image-editor crop photo.png 10 10 200 150
```

The CLI overwrites the target file in place (after `save_as`).

## `edit` library

```rust
use edit::ImageDoc;

fn main() -> edit::Result<()> {
    let mut doc = ImageDoc::open("photo.jpg")?;
    doc.resize(1280, 0, true)?;
    doc.crop(0, 0, 800, 600)?;
    doc.save_as("photo.jpg")?;
    Ok(())
}
```

See `crates/edit` for EXIF upright open, undo cap, and megapixel limits.

## Layout

```
Cargo.toml
crates/edit/                      image library + tests
crates/edit-cli/                  `notia-image-editor` binary
crates/notia-image-editor-plugin/ optional Notia `Plugin` (feature `notia`)
manifest.json + main.js             community plugin at repo root
.github/workflows/ci.yml
```

## Tests

```sh
cargo test --workspace
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

CI builds `edit` and `edit-cli` on Ubuntu, macOS, and Windows. The native plugin is built locally with `--features notia` when a Notia checkout is available.

## License

MIT.