# Building on Fedora

Notes for building the Tauri desktop app (`npm run tauri build` / `npm run tauri dev`)
on Fedora. Verified on Fedora 44. The [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
covers the general case; this page records the two Fedora-specific snags you will hit.

## 1. System dependencies

Tauri v2 needs the GTK 3 / WebKitGTK 4.1 development stack. Install it with:

```bash
sudo dnf install \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libsoup3-devel \
  cairo-devel \
  pango-devel \
  librsvg2-devel \
  glib2-devel
```

If the build stops with a `pkg-config` error like:

```
The system library `gdk-3.0` required by crate `gdk-sys` was not found.
```

the corresponding `-devel` package from the list above is missing. The mapping from the
`.pc` name in the error to the Fedora package:

| Missing `.pc`                        | Fedora package        |
| ------------------------------------ | --------------------- |
| `gtk+-3.0`, `gdk-3.0`                | `gtk3-devel`          |
| `webkit2gtk-4.1`, `javascriptcoregtk-4.1` | `webkit2gtk4.1-devel` |
| `libsoup-3.0`                        | `libsoup3-devel`      |
| `cairo`                              | `cairo-devel`         |
| `pango`                              | `pango-devel`         |
| `librsvg-2.0`                        | `librsvg2-devel`      |
| `glib-2.0`, `gobject-2.0`            | `glib2-devel`         |

## 2. AppImage bundling: build with `NO_STRIP=true`

Once everything compiles, the `.deb` and `.rpm` bundles build fine, but the **AppImage**
step can fail during `linuxdeploy`:

```
ERROR: Strip call failed: .../strip: libwebkit2gtk-4.1.so.0: unknown type [0x13] section `.relr.dyn'
failed to bundle project `failed to run .../linuxdeploy-x86_64.AppImage`
```

This is not a missing dependency. The `strip` binary bundled inside Tauri's `linuxdeploy`
AppImage is old binutils that does not understand the `.relr.dyn` ELF section used by
Fedora's newer system libraries. Tell linuxdeploy to skip stripping:

```bash
NO_STRIP=true npm run tauri build
```

This is an environmental workaround (recent Fedora libraries + the old bundled
`linuxdeploy`), not a project setting, so it is not baked into the repo. If you only need
the `.rpm`/`.deb` you can ignore the AppImage failure entirely.

## 3. The icon when running from the checkout

On Wayland, Plasma and GNOME don't use the icon the window carries. They
find the app's `.desktop` file by its app id, which for Aximar is the program
name, `aximar`. The `.rpm` and `.deb` install `Aximar.desktop` with
`StartupWMClass=aximar` and the icons in the hicolor theme, so an installed
build shows its icon. A build run from `target/` has no desktop file and
shows a generic one. To fix that for your user, add
`~/.local/share/applications/aximar.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Aximar
Exec=/path/to/aximar/target/release/aximar %F
Icon=/path/to/aximar/src-tauri/icons/icon.png
StartupWMClass=aximar
```

## 4. The window's chrome

Linux keeps the system's title bar and GTK menu bar. The tab bar and
toolbar take the menu bar's colours, and the interface takes the desktop's
font, so they match whatever GTK theme is set: Breeze on KDE, Adwaita on
GNOME. See "Colour" in [design-library.md](design-library.md).
