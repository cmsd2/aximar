# aximar-notebook-format

Reads and writes Maxima notebook files (`.macnb`, Jupyter nbformat 4.5) the
way [Aximar](https://github.com/cmsd2/aximar) does. The format is described in
[`docs/notebook-format.md`](https://github.com/cmsd2/aximar/blob/master/docs/notebook-format.md).

One implementation serves every front end: Aximar uses this crate from Rust,
and the VS Code extension uses the same code built as WebAssembly, published
on npm under the same name.

## JavaScript

```js
const { read, write, newId, validId } = require("aximar-notebook-format");

const notebook = read(fs.readFileSync("orbits.macnb", "utf8"));
// notebook.cells[0] → { id, cellType, input, metadata, extra, output }
fs.writeFileSync("orbits.macnb", write(notebook));
```

- `read(json)` returns the cells with their IDs, metadata and outputs. Outputs
  are marked `stale` (they come from an earlier session), and cells with no
  usable ID get new ones (`idsReplaced`). Missing values are `null`. It throws
  on JSON that isn't an nbformat 4 notebook.
- `write(notebook)` returns nbformat 4.5 JSON, byte for byte what Aximar writes.
  Fields you leave out take their defaults.
- `newId()` makes a cell ID; `validId(id)` checks one against nbformat's rule.

Types ship in the package.

## Rust

```rust
let file = aximar_notebook_format::read(&json)?;
let text = aximar_notebook_format::write(&file);
```

## Building and testing

```sh
cargo test -p aximar-notebook-format
wasm-pack build --release --target nodejs --out-dir pkg -- --features wasm
node --test js/smoke.mjs
```

`fixtures/` holds the shared test notebooks; see its README.

## License

MIT
