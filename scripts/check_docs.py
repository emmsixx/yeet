#!/usr/bin/env python3
"""Check Markdown relative file links and agreement between toolchain metadata."""

from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def main():
    errors = []
    files = list(ROOT.glob("*.md"))
    for directory in ("docs", "examples", ".github"):
        files.extend((ROOT / directory).rglob("*.md"))
    for file in files:
        for match in re.finditer(r"\[[^\]]*\]\(([^\s)]+)\)", file.read_text(encoding="utf-8")):
            link = urlsplit(match.group(1))
            if link.scheme or link.netloc or not link.path:
                continue
            target = (file.parent / unquote(link.path)).resolve()
            if not target.is_relative_to(ROOT) or not target.exists():
                errors.append(f"{file.relative_to(ROOT)}: broken local link {match.group(1)}")
    with (ROOT / "Cargo.toml").open("rb") as source:
        package = tomllib.load(source)["package"]
    with (ROOT / "rust-toolchain.toml").open("rb") as source:
        toolchain = tomllib.load(source)["toolchain"]["channel"]
    if package["rust-version"] != toolchain:
        errors.append("update the supported Rust version and pinned toolchain together")
    if package["license"] != "GPL-3.0-or-later":
        errors.append("Cargo license must match the project's GPL-3.0-or-later policy")
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f"Checked local links in {len(files)} Markdown files and package metadata.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
