"""Put small.png into every icon of 64 px or less, after `tauri icon`.

tauri icon scales one image to every size; at 32 px and below the faded
back edges of the full ball blur into noise, so the small sizes use the
front half only.  Needs Pillow; runs iconutil, so macOS only.
"""
import os
import subprocess
import sys
import tempfile

from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
icons = os.path.dirname(here)
big = Image.open(os.path.join(here, "master.png")).convert("RGBA")
small = Image.open(os.path.join(here, "small.png")).convert("RGBA")


def at(n):
    return (small if n <= 64 else big).resize((n, n), Image.LANCZOS)


for name, n in (("32x32.png", 32), ("Square30x30Logo.png", 30), ("Square44x44Logo.png", 44), ("StoreLogo.png", 50)):
    at(n).save(os.path.join(icons, name))

sizes = [16, 24, 32, 48, 64, 256]
imgs = [at(n) for n in sizes]
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
