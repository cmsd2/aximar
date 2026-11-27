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
`small.png`) are build outputs and are not committed.
