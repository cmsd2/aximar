# Homebrew distribution

Aximar is installed via the `cmsd2/homebrew-tap` tap:

```sh
brew install --cask cmsd2/tap/aximar   # GUI app; pulls in aximar-tools
brew install cmsd2/tap/aximar-tools    # CLI tools only
```

The tap's contents are staged in `packaging/homebrew/` — copy them to the root
of the `cmsd2/homebrew-tap` repo. `bump.py` rewrites versions and sha256s from
the latest **published** (non-draft) GitHub releases: `v*` for the cask,
`tools-v*` for the formula. Publishing a draft release is therefore the release
trigger: publishing fires `.github/workflows/notify-tap.yml`, which sends an
`aximar-released` dispatch that starts the tap's `Bump` workflow. The tap also
runs `Bump` daily to catch a missed dispatch, or run it by hand with
`gh workflow run Bump -R cmsd2/homebrew-tap`. `notify-tap.yml` needs a
`TAP_DISPATCH_TOKEN` repository secret (a fine-grained PAT with access to
`cmsd2/homebrew-tap` only, Contents read/write); without it the workflow logs a
warning and the daily run picks the release up.

## macOS signing and notarization

`.github/workflows/release.yml` signs and notarizes the macOS app when these
secrets are set in the `release` GitHub Environment (Settings → Environments,
with required reviewers, so a tag push waits for approval before the macOS jobs
get the secrets); otherwise it builds unsigned (with a warning):

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | base64 of the Developer ID Application `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` password |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID` | Apple ID email |
| `APPLE_PASSWORD` | app-specific password |
| `APPLE_TEAM_ID` | Team ID |

Unsigned builds trigger Gatekeeper warnings on first launch. The app only
spawns external processes (Maxima etc.) and loads no in-process dylibs, so the
hardened runtime needs no extra entitlements.
