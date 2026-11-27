# Licences

Aximar ships code and fonts from open-source packages, and their licences
ask that copies carry each package's copyright notice and licence text.
The built app holds them in `licenses/`.

| File | What it covers | Where it comes from |
| --- | --- | --- |
| `JetBrainsMono-OFL.txt` | The monospace font | `public/licenses/`, copied from `@fontsource/jetbrains-mono` |
| `ComputerModern-OFL.txt` | The Computer Modern Markdown font | `public/licenses/`, copied from `computer-modern` |
| `third-party-npm.txt` | Every npm package in the window's bundle | Written by `npm run build` |

## The accepted licences

`scripts/accepted-licences.ts` lists the licences Aximar accepts, in order
of preference: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
BSD-3-Clause, BSD-2-Clause, ISC, Zlib, 0BSD, MIT-0, Unicode-3.0, CC0-1.0,
BSL-1.0, Unlicense, OFL-1.1 and MPL-2.0.

A package that offers a choice, such as `MIT OR Apache-2.0`, is used under
the first one in that order, and its notice says so. A package that needs
two, such as `MIT AND BSD-3-Clause`, needs both on the list.

MPL-2.0 is on the list because its copyleft covers only the MPL files
themselves: using them unmodified, with a pointer to their source, meets
it. Everything else on the list asks only for the notice.

## The npm notices

`rollup-plugin-license`, in `vite.config.ts`, sees every module Rollup puts
in the window's bundle, and writes `dist/licenses/third-party-npm.txt` from
`scripts/npm-notices.ts`: each package's name and version, the licence
Aximar uses it under, its source, and its licence and notice files. A
package with no licence file gets its copyright line from `package.json`
and, for MIT, the licence's standard text. Packages that only run at build
time, such as Lingui's command line and Babel, never reach the bundle and
aren't listed.

The plugin only sees JavaScript. A package imported only for its CSS and
fonts, such as `@fontsource/jetbrains-mono` and `computer-modern`, needs
its licence copied into `public/licenses/` by hand, and a line in
`public/licenses/README.txt`.

The plugin doesn't run in `vite dev` or in Storybook's build.

### When the build fails on a licence

`npm run build` stops with the package's name and licence when a package in
the bundle has a licence outside the list, or none. Then:

1. Check the package's own repository: a missing or odd `license` field in
   `package.json` is often a packaging slip, and a newer version may fix it.
2. Look for an alternative package under an accepted licence.
3. If the package is the right one and its licence is acceptable, add the
   licence to `ACCEPTED_LICENCES`, with a sentence here on why. That's a
   decision about what Aximar may ship, so it goes in its own commit.
