# Release process

The repository is prepared for GitHub Releases. Nothing is automatically submitted
to a package manager, and crates.io publication is disabled with `publish = false`.
The current version is an unreleased development prerelease.

## CI and repository setup

Pull requests and pushes to `main` run `.github/workflows/ci.yml`. Formatting,
Clippy, Rust docs, source packaging, distribution tests, local doc links, three-OS
Rust tests, and a current-stable compatibility job feed the `CI passed` gate.
Tag releases call the same workflow before building artifacts.

Maintainers must enable GitHub Actions and configure repository rules in GitHub:

- Require pull requests and `CI passed` on `main`; prevent force pushes/deletion.
- Restrict creation and modification of `v*` tags to release maintainers.
- Enable private vulnerability reporting and dependency alerts where available.
- Confirm the repository/account supports the selected hosted runners and artifact
  attestations. ARM Linux runners are intended for a public repository here.
- Limit repository workflow permissions by default; the draft-release job alone
  requests content-write, OIDC, and attestation permissions.

These settings are not applied by checking in YAML. No PAT or Cargo token is
needed for GitHub Releases: the draft job uses the scoped `GITHUB_TOKEN`.
Third-party actions are pinned to full commit SHAs and updated by Dependabot,
following [GitHub's secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use).

## Version and tag

Use SemVer, with `v` in Git tags only. Breaking CLI/config behavior merits a minor
version before 1.0 and a major version after 1.0. Use `-alpha.N` or `-beta.N` for
prereleases. Normal binary updates must not silently migrate configuration.

1. Complete the intended milestone and update `Cargo.toml` and `Cargo.lock` in a PR.
2. Move the relevant `CHANGELOG.md` entries into a dated release section, and update
   installation examples if their version is stale.
3. Run CI and review changes to workflows, the installer, and dependency licenses.
4. Merge and tag that reviewed commit. For example, when the manifest matches:

   ```sh
   git tag -a v0.1.0-alpha.1 -m 'Release v0.1.0-alpha.1'
   git push origin v0.1.0-alpha.1
   ```

Tag creation is the release trigger. The workflow requires an exact manifest/tag
match; mismatched tags fail. Do not retag an already published version.

## What automation produces

The release matrix builds and tests natively for Linux x86_64/ARM64, macOS
x86_64/ARM64, and Windows x86_64. Each release binary passes a version/help smoke
test before packaging. Platform baselines are listed in [installation](installation.md).

`scripts/package.py` creates versioned archives containing the executable, license,
docs, changelog, and config examples, plus per-archive SHA-256 sidecars.
`scripts/prepare_release.py` requires all five archives, verifies every checksum,
and adds the installer, corresponding project source from the tag, and an aggregate
`SHA256SUMS` file. Source includes the Cargo lockfile and build/release scripts;
dependencies are fetched through Cargo using that lockfile.

The draft job generates GitHub build-provenance attestations and creates a **draft**
release. Versions containing a prerelease suffix are marked as prereleases. Review
the notes and download/verify/test the artifacts before publishing the draft.
Attestations establish build provenance; they are not Apple/Windows code signing.

If a workflow fails before the draft is created, rerun the failed jobs after fixing
the cause. If GitHub already contains a partially uploaded draft, remove that draft
and rerun the draft job; inspect its state first. The workflow deliberately fails
instead of overwriting an existing release. Never remove or overwrite public
release assets to fix a bug; publish a new patch version.

## Verify packaging locally

After building a release binary, package it using its actual target:

```sh
cargo build --locked --release --target x86_64-unknown-linux-gnu
python3 scripts/package.py --tag v0.1.0-alpha.1 --target x86_64-unknown-linux-gnu
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Use the matching manifest version and platform target. Packaging alone does not
publish anything. `cargo package --locked` verifies the source distribution.
Installer tests use fake local downloads, so they do not require a published release.

## Package-manager path

Keep the installed binary name `yeet` independent of the Cargo package name. The
working Cargo name `yeet-cli` is not a reserved crates.io name. Before registry
publication, check availability, choose a name, enable publication, and run
`cargo publish --dry-run --locked`. Use registry trusted publishing where supported
when that channel is implemented; do not add a long-lived token speculatively.

For Homebrew, start with a separately maintained tap once stable archives exist.
Its formula should select the platform asset, pin the checksum, include the GPL
license identifier, and test `yeet --version`. Formula changes should be PRs
generated from an already published release. Do not advertise `brew install yeet`
until that formula/channel actually exists.

WinGet/Scoop and Linux distro packages can consume the same immutable archives
and corresponding source. Their version, checksum, ownership, and update behavior
must be controlled by the package manager. Keep self-updates disabled for those
builds. Review each package manager's current submission requirements when adding
that integration; they are not configured here.
