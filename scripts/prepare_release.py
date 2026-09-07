#!/usr/bin/env python3
"""Verify the complete platform set and add checksummed installer/source assets."""

import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess

from package import ROOT, TARGETS, validate_tag


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def verify_archives(directory, tag):
    expected = set()
    for target in TARGETS:
        name = f"yeet-{tag}-{target}.tar.gz"
        archive = directory / name
        checksum = directory / f"{name}.sha256"
        expected.update((name, checksum.name))
        if not archive.is_file() or not checksum.is_file():
            raise ValueError(f"missing release archive or checksum: {name}")
        if checksum.read_text(encoding="utf-8") != f"{digest(archive)}  {name}\n":
            raise ValueError(f"checksum mismatch: {name}")
    actual = {entry.name for entry in directory.iterdir()}
    if actual != expected:
        raise ValueError(f"unexpected release files: {sorted(actual - expected)}")


def prepare(directory, tag):
    validate_tag(tag)
    verify_archives(directory, tag)
    shutil.copyfile(ROOT / "install.sh", directory / "install.sh")
    # Ship corresponding project source, including build/release scripts and lockfile.
    # Dependencies remain identified in Cargo.lock and fetched through Cargo.
    subprocess.run(
        ["git", "archive", "--format=tar.gz", f"--prefix=yeet-{tag}/",
         "-o", str((directory / f"yeet-{tag}-source.tar.gz").resolve()), tag],
        cwd=ROOT, check=True,
    )
    for name in ("install.sh", f"yeet-{tag}-source.tar.gz"):
        (directory / f"{name}.sha256").write_text(
            f"{digest(directory / name)}  {name}\n", encoding="utf-8"
        )
    manifest = "".join(
        file.read_text(encoding="utf-8") for file in sorted(directory.glob("*.sha256"))
    )
    (directory / "SHA256SUMS").write_text(manifest, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--directory", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    try:
        prepare(args.directory, args.tag)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"release preparation failed: {error}\n")


if __name__ == "__main__":
    main()
