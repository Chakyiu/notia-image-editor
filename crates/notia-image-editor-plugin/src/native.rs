//! Rust `Plugin` implementation when built with `--features notia`.

extern crate gpui_kit as gpui;

use gpui::{App, Window};
use gpui::component::WindowExt as _;
use notia_plugin::{Command, Notia, Plugin, PluginContext, PluginManifest, core::VaultPath};

use crate::image_path::{image_target_from_line, is_supported_image_path};

pub struct ImageEditorPlugin;

fn embedded_image_at_cursor(cx: &App) -> Option<VaultPath> {
    let workspace = Notia::workspace(cx);
    let active = workspace.read(cx).active_note()?;
    let line = active.cursor_line() as usize;
    let text = active.text();
    let target = text
        .lines()
        .nth(line)
        .and_then(image_target_from_line)?;
    let from = active.path().clone();
    let vault = Notia::vault(cx);
    let resolved = vault.read(cx).index().resolve(&target, Some(&from))?;
    if is_supported_image_path(resolved.as_str()) {
        Some(resolved)
    } else {
        None
    }
}

fn apply_resize(path: &VaultPath, width: u32, window: &mut Window, cx: &mut App) {
    let vault = Notia::vault(cx);
    let abs = vault.read(cx).vault().abs_path(path);
    let path = path.clone();
    let result = (|| -> edit::Result<()> {
        let mut doc = edit::ImageDoc::open(&abs)?;
        doc.resize(width, 0, true)?;
        doc.save_as(&abs)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            vault.update(cx, |model, cx| model.rescan(cx));
            window.push_notification(
                format!("Resized “{}” to width {width}px", path.as_str()),
                cx,
            );
        }
        Err(error) => {
            window.push_notification(format!("Could not resize image: {error}"), cx);
        }
    }
}

fn apply_crop(path: &VaultPath, window: &mut Window, cx: &mut App) {
    let vault = Notia::vault(cx);
    let abs = vault.read(cx).vault().abs_path(path);
    let path = path.clone();
    let result = (|| -> edit::Result<()> {
        let mut doc = edit::ImageDoc::open(&abs)?;
        let (w, h) = (doc.width(), doc.height());
        let margin = (w.min(h) / 10).max(1);
        doc.crop(margin, margin, w - 2 * margin, h - 2 * margin)?;
        doc.save_as(&abs)?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            vault.update(cx, |model, cx| model.rescan(cx));
            window.push_notification(format!("Cropped “{}”", path.as_str()), cx);
        }
        Err(error) => {
            window.push_notification(format!("Could not crop image: {error}"), cx);
        }
    }
}

impl Plugin for ImageEditorPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new("notia-image-editor", "Image Editor")
            .with_description("Crop and resize images embedded in notes.")
    }

    fn activate(&self, ctx: &mut PluginContext, _cx: &mut App) {
        ctx.add_command(
            Command::new(
                "resize-embedded",
                "Resize embedded image at cursor (width 1280px)",
                move |window, cx| {
                    if let Some(path) = embedded_image_at_cursor(cx) {
                        apply_resize(&path, 1280, window, cx);
                    } else {
                        window.push_notification(
                            "Put the cursor on a line with an embedded image.",
                            cx,
                        );
                    }
                },
            )
            .available_when(|cx| embedded_image_at_cursor(cx).is_some()),
        );

        ctx.add_command(
            Command::new(
                "crop-embedded",
                "Crop embedded image at cursor (10% margin)",
                move |window, cx| {
                    if let Some(path) = embedded_image_at_cursor(cx) {
                        apply_crop(&path, window, cx);
                    } else {
                        window.push_notification(
                            "Put the cursor on a line with an embedded image.",
                            cx,
                        );
                    }
                },
            )
            .available_when(|cx| embedded_image_at_cursor(cx).is_some()),
        );
    }
}