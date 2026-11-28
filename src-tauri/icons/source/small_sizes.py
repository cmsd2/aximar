"""Finish the icons after `tauri icon`: small sizes and the Windows files.

tauri icon scales one image to every size; at 32 px and below the faded
back edges of the full ball blur into noise, so the small sizes use the
front half only.

Windows and Linux draw icons edge to edge, so icon.ico, the Square*Logo
tiles and the PNGs Linux uses come from the *_win drawings, cropped to the
tile with no shadow.  The macOS drawings' margin would make them look too
small on the taskbar.  Linux's PNGs are the window's icon (the first PNG
in tauri.conf.json's list) and the sizes a package installs in hicolor.

Needs Pillow.  icon.icns needs iconutil, so it is only rebuilt on macOS.
"""
import os
import subprocess
import sys
import tempfile

from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
icons = os.path.dirname(here)


def load(name):
    return Image.open(os.path.join(here, name + ".png")).convert("RGBA")


mac = (load("small"), load("master"))
win = (load("small_win"), load("master_win"))


def at(n, drawings=mac):
    small, big = drawings
    return (small if n <= 64 else big).resize((n, n), Image.LANCZOS)


linux = {"32x32.png": 32, "64x64.png": 64, "128x128.png": 128, "128x128@2x.png": 256, "256x256.png": 256, "icon.png": 512}
for name, n in linux.items():
    at(n, win).save(os.path.join(icons, name))

tiles = {f"Square{n}x{n}Logo.png": n for n in (30, 44, 71, 89, 107, 142, 150, 284, 310)}
tiles["StoreLogo.png"] = 50
for name, n in tiles.items():
    at(n, win).save(os.path.join(icons, name))

sizes = [16, 24, 32, 48, 64, 256]
imgs = [at(n, win) for n in sizes]
imgs[-1].save(os.path.join(icons, "icon.ico"), sizes=[(n, n) for n in sizes], append_images=imgs[:-1])

if sys.platform == "darwin":
    iconset = os.path.join(tempfile.mkdtemp(), "icon.iconset")
    os.makedirs(iconset)
    for pt in (16, 32, 128, 256, 512):
        at(pt).save(os.path.join(iconset, f"icon_{pt}x{pt}.png"))
        at(pt * 2).save(os.path.join(iconset, f"icon_{pt}x{pt}@2x.png"))
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", os.path.join(icons, "icon.icns")], check=True)
else:
    print("Not macOS: icon.icns left as tauri icon made it")
