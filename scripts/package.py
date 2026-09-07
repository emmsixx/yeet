#!/usr/bin/env python3
"""Build release archives from already-built binaries. Python 3.11+, stdlib only."""

import argparse
import hashlib
from pathlib import Path
import re
import shutil
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


def version():
    with (ROOT / "Cargo.toml").open("rb") as source:
        return tomllib.load(source)["package"]["version"]


def validate_tag(tag):
    expected = f"v{version()}"
    if tag != expected or not re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError(f"release tag must match Cargo.toml exactly: {expected}")


def package(target, tag, binary, output):
    validate_tag(tag)
    if target not in TARGETS:
        raise ValueError(f"unsupported target: {target}")
    if not binary.is_file():
        raise ValueError(f"missing release binary: {binary}")
    output.mkdir(parents=True, exist_ok=True)
    name = f"yeet-{tag}-{target}"
    windows = target.endswith("windows-msvc")
    archive = output / f"{name}{'.zip' if windows else '.tar.gz'}"
    with tempfile.TemporaryDirectory(prefix="yeet-package-") as temporary:
        bundle = Path(temporary) / name
        bundle.mkdir()
        executable = bundle / ("yeet.exe" if windows else "yeet")
        shutil.copyfile(binary, executable)
        executable.chmod(0o755)
        for filename in ("README.md", "LICENSE", "CHANGELOG.md", "CONTRIBUTING.md", "SECURITY.md"):
            shutil.copyfile(ROOT / filename, bundle / filename)
        shutil.copytree(ROOT / "docs", bundle / "docs")
        shutil.copytree(ROOT / "examples", bundle / "examples")
        if windows:
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as destination:
                for entry in sorted(bundle.rglob("*")):
                    if entry.is_file():
                        destination.write(entry, entry.relative_to(Path(temporary)))
        else:
            with tarfile.open(archive, "w:gz") as destination:
                destination.add(bundle, arcname=name)
    with archive.open("rb") as source:
        checksum = hashlib.file_digest(source, "sha256").hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(
        f"{checksum}  {archive.name}\n", encoding="utf-8"
    )
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    try:
        validate_tag(args.tag)
        if args.target:
            binary = args.binary or ROOT / "target" / args.target / "release" / (
                "yeet.exe" if args.target.endswith("windows-msvc") else "yeet"
            )
            print(package(args.target, args.tag, binary, args.output))
        elif args.binary:
            parser.error("--binary requires --target")
    except ValueError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
