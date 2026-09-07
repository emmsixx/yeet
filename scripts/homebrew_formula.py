#!/usr/bin/env python3
"""Generate a tap formula from a verified Yeet release checksum manifest."""

import argparse
from pathlib import Path
import re

from package import TARGETS, validate_tag


def formula(tag, checksums):
    validate_tag(tag)
    entries = {}
    for line in checksums.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", line)
        if not match or match[2] in entries:
            raise ValueError("malformed or duplicate checksum entry")
        entries[match[2]] = match[1]

    assets = {}
    for target in TARGETS:
        if target.endswith("windows-msvc"):
            continue
        name = f"yeet-{tag}-{target}.tar.gz"
        if name not in entries:
            raise ValueError(f"missing checksum for {name}")
        assets[target] = (name, entries[name])

    lines = [
        "class Yeet < Formula",
        '  desc "Git workflow CLI with configurable commit-message generation"',
        '  homepage "https://github.com/emmsixx/yeet"',
        f'  version "{tag[1:]}"',
        '  license "GPL-3.0-or-later"',
    ]
    for operating_system, suffix in (("macos", "apple-darwin"), ("linux", "unknown-linux-gnu")):
        lines.extend(["", f"  on_{operating_system} do"])
        if operating_system == "macos":
            lines.extend(["    depends_on macos: :sequoia", ""])
        for index, (cpu, architecture) in enumerate((("arm", "aarch64"), ("intel", "x86_64"))):
            if index:
                lines.append("")
            name, checksum = assets[f"{architecture}-{suffix}"]
            lines.extend([
                f"    on_{cpu} do",
                f'      url "https://github.com/emmsixx/yeet/releases/download/{tag}/{name}"',
                f'      sha256 "{checksum}"',
                "    end",
            ])
        lines.append("  end")
    lines.extend([
        "", "  def install", '    bin.install "yeet"',
        '    doc.install "README.md", "CHANGELOG.md", "CONTRIBUTING.md", "SECURITY.md", "docs", "examples"',
        "  end", "", "  test do",
        '    assert_match version.to_s, shell_output("#{bin}/yeet --version")',
        '    system bin/"yeet", "--config", testpath/"config.toml", "config", "init"',
        '    assert_path_exists testpath/"config.toml"',
        '    assert_path_exists testpath/"instructions.md"',
        "  end", "end", "",
    ])
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--checksums", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        content = formula(args.tag, args.checksums.read_text(encoding="utf-8"))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content, encoding="utf-8")
    except (ValueError, OSError) as error:
        parser.exit(1, f"cannot generate Homebrew formula: {error}\n")


if __name__ == "__main__":
    main()
