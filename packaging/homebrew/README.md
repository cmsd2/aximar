# homebrew-tap

Homebrew tap for [Aximar](https://github.com/cmsd2/aximar).

```sh
brew install --cask cmsd2/tap/aximar   # GUI app + the tools below
brew install cmsd2/tap/aximar-tools    # just maxima-lsp, maxima-dap, aximar-mcp
```

`bump.py` keeps both files in step with the latest published GitHub releases
(`v*` for the app, `tools-v*` for the tools). The `Bump aximar` workflow runs
it hourly and on `repository_dispatch` (`aximar-released`).
