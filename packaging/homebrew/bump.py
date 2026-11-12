#!/usr/bin/env python3
"""Update Formula/aximar-tools.rb and Casks/aximar.rb to the latest published
(non-draft, non-prerelease) cmsd2/aximar releases. Stdlib only.

Prints the names of changed files; exits 0 either way."""
import hashlib, json, re, sys, urllib.request
from pathlib import Path

REPO = "cmsd2/aximar"
ROOT = Path(__file__).resolve().parent


def get(url):
    with urllib.request.urlopen(url) as r:
        return r.read()


def sha256(url):
    return hashlib.sha256(get(url)).hexdigest()


def latest(releases, prefix):
    for r in releases:  # API returns newest first
        if not r["draft"] and not r["prerelease"] and r["tag_name"].startswith(prefix):
            return r["tag_name"][len(prefix):]
    sys.exit(f"no published release with tag prefix {prefix!r}")


def rewrite(path, fn):
    old = path.read_text()
    new = fn(old)
    if new != old:
        path.write_text(new)
        print(path.relative_to(ROOT))


def main():
    releases = json.loads(get(f"https://api.github.com/repos/{REPO}/releases?per_page=50"))
    base = f"https://github.com/{REPO}/releases/download"

    # --- tools formula: every url line is followed by its sha256 line ---
    tv = latest(releases, "tools-v")

    def formula(text):
        text = re.sub(r'(?m)^(\s*version )"[^"]*"', rf'\1"{tv}"', text)

        def url_sub(m):
            asset = m.group(2)
            url = f"{base}/tools-v{tv}/{asset}"
            return f'{m.group(1)}"{url}"\n{m.group(3)}sha256 "{sha256(url)}"'

        return re.sub(
            r'(\s*url )"[^"]*/(aximar-tools-[^"/]+)"\n(\s*)sha256 "[0-9a-f]*"',
            url_sub, text)

    rewrite(ROOT / "Formula/aximar-tools.rb", formula)

    # --- GUI cask ---
    gv = latest(releases, "v")

    def cask(text):
        arm = sha256(f"{base}/v{gv}/Aximar_{gv}_aarch64.dmg")
        intel = sha256(f"{base}/v{gv}/Aximar_{gv}_x64.dmg")
        text = re.sub(r'(?m)^(\s*version )"[^"]*"', rf'\1"{gv}"', text)
        text = re.sub(r'arm:(\s+)"[0-9a-f]*"', rf'arm:\1"{arm}"', text)
        text = re.sub(r'intel:(\s+)"[0-9a-f]*"', rf'intel:\1"{intel}"', text)
        return text

    rewrite(ROOT / "Casks/aximar.rb", cask)


main()
