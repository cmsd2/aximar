#!/usr/bin/env python3
"""
Aximar release automation.

Walks through the release process one step at a time, pausing between
steps so you can review changes and commit with a suitable message.

Usage:
    python tools/release.py 0.12.0
    python tools/release.py 0.12.0 --dry-run
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

VERSION_FILES_TOML = [
    ROOT / "src-tauri" / "Cargo.toml",
    ROOT / "crates" / "aximar-core" / "Cargo.toml",
    ROOT / "crates" / "aximar-mcp" / "Cargo.toml",
]
VERSION_FILES_JSON = [
    ROOT / "package.json",
    ROOT / "src-tauri" / "tauri.conf.json",
]

SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True, **kwargs)


def current_version() -> str:
    """Read the current version from package.json."""
    data = json.loads((ROOT / "package.json").read_text())
    return data["version"]


def git_branch() -> str:
    return run(["git", "rev-parse", "--abbrev-ref", "HEAD"]).stdout.strip()


def git_is_clean(untracked: bool = True) -> bool:
    cmd = ["git", "status", "--porcelain"]
    if not untracked:
        cmd.append("--untracked-files=no")
    result = run(cmd)
    return result.stdout.strip() == ""


def git_tag_exists(tag: str) -> bool:
    result = subprocess.run(
        ["git", "tag", "-l", tag], cwd=ROOT, capture_output=True, text=True
    )
    return tag in result.stdout.strip().splitlines()


def bump_toml(path: Path, old: str, new: str, dry_run: bool):
    text = path.read_text()
    # Only replace the first occurrence (the [package] version)
    updated = text.replace(f'version = "{old}"', f'version = "{new}"', 1)
    if updated == text:
        print(f"  WARNING: version {old} not found in {path.relative_to(ROOT)}")
        return
    if not dry_run:
        path.write_text(updated)
    print(f"  {path.relative_to(ROOT)}")


def bump_json(path: Path, old: str, new: str, dry_run: bool):
    text = path.read_text()
    updated = text.replace(f'"version": "{old}"', f'"version": "{new}"', 1)
    if updated == text:
        print(f"  WARNING: version {old} not found in {path.relative_to(ROOT)}")
        return
    if not dry_run:
        path.write_text(updated)
    print(f"  {path.relative_to(ROOT)}")


def pause(message: str):
    """Pause and wait for the user to press Enter."""
    print()
    try:
        input(f">>> {message} Press Enter to continue (Ctrl+C to abort)... ")
    except KeyboardInterrupt:
        print("\nAborted.")
        sys.exit(1)
    print()


def step_header(n: int, title: str):
    print(f"\n{'='*60}")
    print(f"  Step {n}: {title}")
    print(f"{'='*60}\n")


def main():
    parser = argparse.ArgumentParser(description="Aximar release automation")
    parser.add_argument("version", help="New version (e.g. 0.12.0)")
    parser.add_argument("--dry-run", action="store_true", help="Show what would happen without making changes")
    args = parser.parse_args()

    new_version = args.version
    dry_run = args.dry_run

    if not SEMVER_RE.match(new_version):
        print(f"Error: '{new_version}' is not a valid semver version (expected X.Y.Z)")
        sys.exit(1)

    old_version = current_version()
    tag = f"v{new_version}"

    print(f"Release: {old_version} -> {new_version}")
    if dry_run:
        print("(dry run — no changes will be made)\n")

    # ── Preflight checks ────────────────────────────────────────────

    step_header(0, "Preflight checks")

    branch = git_branch()
    print(f"  Branch: {branch}")
    if branch != "master":
        print(f"  WARNING: Not on master (on '{branch}')")
        pause("Continue anyway?")

    if not git_is_clean(untracked=False):
        print("  ERROR: Working tree has uncommitted changes. Commit or stash first.")
        sys.exit(1)
    print("  Working tree: clean")

    if git_tag_exists(tag):
        print(f"  ERROR: Tag '{tag}' already exists.")
        sys.exit(1)
    print(f"  Tag {tag}: available")

    if old_version == new_version:
        print(f"  ERROR: New version is the same as current version ({old_version})")
        sys.exit(1)
    print(f"  Version: {old_version} -> {new_version}")

    # ── Step 1: Bump versions ───────────────────────────────────────

    step_header(1, "Bump version numbers")

    print("Updating files:")
    for path in VERSION_FILES_TOML:
        bump_toml(path, old_version, new_version, dry_run)
    for path in VERSION_FILES_JSON:
        bump_json(path, old_version, new_version, dry_run)

    if not dry_run:
        print("\nUpdating Cargo.lock...")
        run(["cargo", "check", "--workspace"])
        print("  Cargo.lock updated")

    # ── Step 2: Commit ──────────────────────────────────────────────

    step_header(2, "Commit version bump")

    files = [
        "src-tauri/Cargo.toml",
        "crates/aximar-core/Cargo.toml",
        "crates/aximar-mcp/Cargo.toml",
        "package.json",
        "src-tauri/tauri.conf.json",
        "Cargo.lock",
    ]

    print("Suggested commit:")
    print(f"  git add {' '.join(files)}")
    print(f'  git commit -m "Bump version to {new_version}"')

    if dry_run:
        print("\n(dry run — skipping commit)")
    else:
        pause("Review the changes, then commit and press Enter.")

        if not git_is_clean(untracked=False):
            print("  ERROR: Uncommitted changes remain. Please commit before continuing.")
            sys.exit(1)

    # ── Step 3: Tag ─────────────────────────────────────────────────

    step_header(3, "Create git tag")

    print(f"  Tag: {tag}")

    if dry_run:
        print("(dry run — skipping tag)")
    else:
        run(["git", "tag", tag])
        print(f"  Created tag {tag}")

    # ── Step 4: Push ────────────────────────────────────────────────

    step_header(4, "Push to remote")

    print(f"  git push && git push origin {tag}")

    if dry_run:
        print("\n(dry run — skipping push)")
    else:
        pause("Ready to push?")
        run(["git", "push"])
        run(["git", "push", "origin", tag])
        print(f"  Pushed commit and tag {tag}")

    # ── Done ────────────────────────────────────────────────────────

    print(f"\n{'='*60}")
    print(f"  Release {tag} complete!")
    print(f"{'='*60}\n")


if __name__ == "__main__":
    main()
