import { Plugin, Notice, ui } from "notia";

const IMAGE_EXT = /\.(png|jpe?g|gif|webp)$/i;

function imageTargetFromLine(line) {
  if (!line) {
    return null;
  }
  const md = line.match(/!\[[^\]]*\]\(([^)]+)\)/);
  if (md) {
    const path = md[1].trim().replace(/^["']|["']$/g, "");
    return IMAGE_EXT.test(path) ? path : null;
  }
  const wiki = line.match(/!\[\[([^|\]]+)/);
  if (wiki) {
    const path = wiki[1].trim();
    return IMAGE_EXT.test(path) ? path : null;
  }
  return null;
}

function embeddedImageAtCursor(app) {
  const note = app.workspace.getActiveNote();
  if (!note) {
    return null;
  }
  const line = note.text.split("\n")[note.line] ?? "";
  const target = imageTargetFromLine(line);
  if (!target) {
    return null;
  }
  return app.metadata.resolveLink(target, note.path) ?? target;
}

function runNative(app, id) {
  app.workspace.runCommand(`notia-image-editor:${id}`);
}

export default class ImageEditorPlugin extends Plugin {
  async onload() {
    const applyResize = () => {
      const path = embeddedImageAtCursor(this.app);
      if (!path) {
        new Notice("Place the cursor on a line with an embedded image.");
        return;
      }
      runNative(this.app, "resize-embedded");
    };

    const applyCrop = () => {
      const path = embeddedImageAtCursor(this.app);
      if (!path) {
        new Notice("Place the cursor on a line with an embedded image.");
        return;
      }
      runNative(this.app, "crop-embedded");
    };

    this.addPanel({
      id: "image-editor",
      title: "Image Editor",
      icon: "crop",
      render: () => {
        const path = embeddedImageAtCursor(this.app);
        const ready = path
          ? `Image: ${path}`
          : "Open a note; put the cursor on an embedded image line.";
        return [
          ui.heading("Image Editor"),
          ui.text(ready, { muted: !path }),
          ui.text(
            "MVP: resize to 1280px width (aspect locked) or crop a 10% margin. Requires the native companion built from this repo — see README.",
            { small: true, muted: true },
          ),
          ui.row([
            ui.button("Resize", applyResize, { variant: "primary", icon: "scaling" }),
            ui.button("Crop", applyCrop, { icon: "crop" }),
          ]),
        ];
      },
    });

    this.addSettingTab({
      render: () => [
        ui.heading("Install"),
        ui.text(
          "Symlink this repository into your vault's .notia/plugins folder, enable the plugin here, then add the native companion to Notia per README (one-time compile) so edits write back to vault files.",
          { muted: true },
        ),
      ],
    });
  }
}