# Releasing

1. Update `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` for the release.
2. Merge after `CI passed` succeeds.
3. Tag that commit with the exact manifest version and push the tag:

   ```sh
   git tag -a v0.1.0-alpha.1 -m 'Release v0.1.0-alpha.1'
   git push origin v0.1.0-alpha.1
   ```

4. Wait for the Release workflow, test the downloaded artifacts, and publish
   the draft on GitHub. Never replace a published version; release a new one.

## Automation

CI runs Rust tests on macOS and Linux, stable-Rust compatibility, formatting,
Clippy, docs, packaging, and installer tests. Configure `CI passed` as a required
check in GitHub; repository rules are not set by workflow YAML.

The release workflow builds and tests four targets: macOS and Linux on x86_64
and ARM64. It creates archives, SHA-256 checksums, source from the tag, an
installer, provenance attestations, and a draft release. Prerelease tags produce
prereleases. Only the draft job has write and attestation permissions.

`scripts/prepare_release.py` requires all four archives and verifies their
checksums. If a run fails, fix the cause and rerun; inspect any existing draft
before retrying. Public assets must remain unchanged.

## Local packaging

```sh
cargo build --locked --release --target x86_64-unknown-linux-gnu
python3 scripts/package.py --tag v0.1.0-alpha.1 --target x86_64-unknown-linux-gnu
```

Use the actual version and target. See [contributing](../CONTRIBUTING.md) for checks.

## Package managers

Homebrew tap formulas can be generated with `scripts/homebrew_formula.py` from a
release checksum manifest. Publish the release before updating the tap.
Cargo publishing remains disabled; verify the registry name before enabling it.
Package-manager installations should update through their package manager.
