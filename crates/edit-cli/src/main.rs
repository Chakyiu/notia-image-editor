//! Apply crop or resize to an image file on disk (used by the Notia community plugin workflow).

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use edit::ImageDoc;

#[derive(Parser)]
#[command(
    name = "notia-image-editor",
    about = "Crop or resize png/jpeg/webp images (notia-image-editor)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resize to a target width (height follows aspect ratio when --lock-aspect is set).
    Resize {
        /// Image file path.
        path: PathBuf,
        /// Target width in pixels.
        width: u32,
        /// Target height in pixels (ignored when --lock-aspect).
        #[arg(long)]
        height: Option<u32>,
        /// Keep aspect ratio from width.
        #[arg(long, default_value_t = true)]
        lock_aspect: bool,
    },
    /// Crop to a rectangle inside the image.
    Crop {
        /// Image file path.
        path: PathBuf,
        /// Left offset in pixels.
        x: u32,
        /// Top offset in pixels.
        y: u32,
        /// Crop width in pixels.
        width: u32,
        /// Crop height in pixels.
        height: u32,
    },
    /// Print image dimensions.
    Info { path: PathBuf },
}

fn main() -> edit::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Resize {
            path,
            width,
            height,
            lock_aspect,
        } => {
            let mut doc = ImageDoc::open(&path)?;
            let height = height.unwrap_or(0);
            doc.resize(width, height, lock_aspect)?;
            doc.save_as(&path)?;
            eprintln!("resized {:?} to {}×{}", path, doc.width(), doc.height());
        }
        Command::Crop {
            path,
            x,
            y,
            width,
            height,
        } => {
            let mut doc = ImageDoc::open(&path)?;
            doc.crop(x, y, width, height)?;
            doc.save_as(&path)?;
            eprintln!("cropped {:?} to {}×{}", path, doc.width(), doc.height());
        }
        Command::Info { path } => {
            let doc = ImageDoc::open(&path)?;
            println!("{}×{}", doc.width(), doc.height());
        }
    }
    Ok(())
}
