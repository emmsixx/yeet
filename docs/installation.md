# Installation and updates

**No release is published yet.** Build from source today. The release instructions
below describe the checked-in distribution pipeline and become usable after a
maintainer publishes a release.

## From source

Install rustup and a native linker, clone the repository, and run:

```sh
cargo install --path . --locked
yeet --version
```

Run this from the checkout. It installs to Cargo's bin directory. The pinned Rust
toolchain and committed lockfile are used for the build. This is not yet a
`cargo install yeet-cli` registry package: publication is disabled, and the
registry name has not been reserved or verified.

## Release installer: Linux and macOS

After a release is published, download its versioned installer, inspect it, and
run it. Replace the example version with the release you want:

```sh
version=v0.1.0-alpha.1
curl --proto '=https' --tlsv1.2 -fsSL \
  "https://github.com/emmsixx/yeet/releases/download/$version/install.sh" \
  -o install-yeet.sh
less install-yeet.sh
sh install-yeet.sh --version "$version"
```

The default directory is `~/.local/bin`. Use `--bin-dir /absolute/path` or
`YEET_INSTALL_DIR` to change it. `--version` overrides `YEET_VERSION`; without
either, the installer resolves the latest stable release. Prereleases require an
explicit version, including when there are no stable releases yet.

The installer downloads the matching archive and SHA-256 checksum over HTTPS,
checks the checksum, extracts the binary into a temporary directory, and checks
its version. It then atomically replaces the destination binary. Failures before
replacement preserve the existing binary. It refuses a symlink destination.
It does not invoke sudo or modify shell startup files. Add the chosen bin directory
to PATH yourself if needed.

Requirements: a POSIX shell, curl, tar, common Unix utilities, and either
`sha256sum` or `shasum`. Download or execution failures are reported directly.

## Manual release archives

Download the archive for your platform and its `.sha256` sidecar from the same
GitHub Release. On Linux run `sha256sum -c ARCHIVE.sha256`; on macOS run
`shasum -a 256 -c ARCHIVE.sha256`. Replace `ARCHIVE` with the full filename.
Extract the archive and place the binary in a directory on PATH.

On Windows download the x86_64 ZIP, compare `Get-FileHash -Algorithm SHA256` with
its sidecar, expand the archive, and place `yeet.exe` in a user-owned directory
on PATH. There is no PowerShell installer yet.

| Platform | Release target | Build/test baseline |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Ubuntu 22.04, glibc 2.35 |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Ubuntu 22.04 ARM, glibc 2.35 |
| macOS Intel | `x86_64-apple-darwin` | macOS 15 Intel |
| macOS Apple Silicon | `aarch64-apple-darwin` | macOS 15 ARM64 |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Windows Server 2022 runner |

These are CI baselines, not a tested guarantee for all older OS versions. Alpine
and other musl systems need a source build; static musl assets are future work.
Windows ARM64 is not currently packaged. Apple notarization and Windows code
signing are not configured.

Release filenames are stable: `yeet-vVERSION-TARGET.tar.gz` or `.zip`, containing
a matching top-level directory with the binary, GPL license, docs, and examples.
Each release also carries `SHA256SUMS`, the installer, and a source archive.

For independent provenance verification, use GitHub CLI:

```sh
gh attestation verify ARCHIVE --repo emmsixx/yeet
```

The installer verifies checksums but does not perform this attestation check.
GitHub explains how verification identifies an artifact's repository and build
workflow in its [artifact attestation documentation](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).

## Upgrading, rollback, and uninstalling

- **Script install:** rerun the installer with the desired version. To roll back,
  select the older release explicitly. Avoid using this to replace a binary owned
  by a package manager; regular files are not yet identified by install ownership.
- **Manual archive:** replace the binary with a verified archive of the chosen version.
- **Cargo source install:** update your checkout and repeat `cargo install --path . --locked --force`.
- **Future package-manager install:** use that package manager's upgrade command.

No background updating is implemented. See [update design](updates.md).

To uninstall a script/archive install, remove the `yeet` binary from its install
directory. For Cargo use `cargo uninstall yeet-cli`. Keep `~/.config/yeet` unless
you also want to remove your future settings and instructions. Homebrew, WinGet,
and other package managers are not published channels yet.
