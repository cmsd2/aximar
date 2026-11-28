# App icon

Aximar's icon is the Cayley graph of the alternating group
A5 = ⟨a, b | a⁵ = b² = (ab)³ = 1⟩. The generator `a` traces the 12 ink
pentagons and `b` the 30 vermilion seams; the graph is the truncated
icosahedron, a football, seen with one vertex facing the viewer.
`make_icon.py` explains the construction.

Two drawings:

- `master.svg`: the whole ball, back edges fading and thinning with depth.
  Used for 128 px and up.
- `small.svg`: the front half only, with heavier strokes. Used for 64 px
  and under, where the back edges would blur into noise.

Each has a Windows copy, `master_win.svg` and `small_win.svg`. The macOS
drawings follow Apple's grid, an 824 px tile with a drop shadow in a
1024 px canvas. Windows draws an icon edge to edge, so with that margin
the icon looks a fifth too small on the taskbar. The Windows copies crop
the canvas to the tile and have no shadow. Most Windows icons are bare
shapes that reach the edges, and next to them the pale tile reads as
background, so the Windows copies also enlarge the ball to nearly fill
the tile. Linux draws icons edge to edge too, so the Windows copies make
`icon.ico`, the `Square*Logo.png` tiles, and the PNGs Linux uses:
`icon.png` and the sizes `tauri.conf.json` lists, which a Linux package
installs in the hicolor theme. The first PNG in that list is also the
window's icon. Only `icon.icns` uses the macOS drawings.

## Rebuilding the icons

From the repo root, on macOS:

```bash
python3 src-tauri/icons/source/make_icon.py
node src-tauri/icons/source/render.mjs
npx tauri icon src-tauri/icons/source/master.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/64x64.png
python3 src-tauri/icons/source/small_sizes.py
```

`small_sizes.py` needs Pillow. The PNGs it reads (`master.png`,
`small.png`, `master_win.png`, `small_win.png`) are build outputs and are
not committed.

The same steps run on Windows, except that `icon.icns` keeps the version
`tauri icon` made, since rebuilding it needs macOS's `iconutil`. To change
only the Windows and Linux files, skip `tauri icon` and the `rm`:
`small_sizes.py` rebuilds `icon.ico`, the tiles and the Linux PNGs on its
own.

On Linux, Plasma and GNOME on Wayland don't use the window's icon. They
find the app's `.desktop` file by its app id, which is the program name,
`aximar`. A package installs one. A build run from the checkout has none,
so it shows a generic icon until you add one (see
docs/building-on-fedora.md). If `render.mjs` can't find
Playwright's Chromium, launch Edge instead with
`chromium.launch({ channel: "msedge" })`.
