# Install and update

## From source

Requires Git, Rust, and a native linker on macOS or Linux:

```sh
git clone https://github.com/emmsixx/yeet.git
cd yeet
cargo install --path . --locked
```

The checkout pins Rust 1.94.1. To update, pull the latest source and rerun
`cargo install --path . --locked --force`. Uninstall with `cargo uninstall yeet-cli`.

## Release installer

Once a release is published, download its installer and select a version:

```sh
version=v0.1.0-alpha.1
curl -fsSL "https://github.com/emmsixx/yeet/releases/download/$version/install.sh" -o install-yeet.sh
sh install-yeet.sh --version "$version"
```

The installer checks SHA-256 and installs to `~/.local/bin`; add that to PATH.
Use `--bin-dir PATH` to change the destination. It requires curl, tar, and
`sha256sum` or `shasum`. Rerun for updates or rollback; remove the binary to uninstall.
Keep `~/.config/yeet` to preserve your settings.

## Release archives

| Platform | Target | Build baseline |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Ubuntu 22.04, glibc 2.35 |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Ubuntu 22.04, glibc 2.35 |
| macOS Intel | `x86_64-apple-darwin` | macOS 15 |
| macOS Apple Silicon | `aarch64-apple-darwin` | macOS 15 |

Download `yeet-vVERSION-TARGET.tar.gz` and its `.sha256` sidecar from the same
release. Verify with `sha256sum -c ARCHIVE.sha256` on Linux or
`shasum -a 256 -c ARCHIVE.sha256` on macOS, then extract `yeet` into PATH.
Optional provenance check: `gh attestation verify ARCHIVE --repo emmsixx/yeet`.

Older OS versions are unverified; musl systems need a source build.
Apple notarization is not configured. Package-manager distribution is planned.
