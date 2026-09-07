# Roadmap

## Present in the repository

- Rust package, reusable library, CLI, lockfile, and toolchain pin.
- CLI smoke tests and packaging/installer failure-path tests.
- CI, Dependabot, four-target release workflow, checksums, provenance, source
  archives, and a checksum-verifying Unix installer.
- GPL-3.0-or-later license, README, contribution guidance, and documentation.

Workflows still need to run on GitHub; release assets need an actual published tag
before users can install them.

## First usable release — implemented

- Typed configuration, profile resolution, and config init/show commands.
- Manual commit workflow with stage-all, preview, confirmation, `--yes`, and
  `--no-push`, tested against temporary repositories and a local bare remote.
- Repository, Conventional Commit, and custom writing styles.
- Codex adapter with bounded context, structured output, and cancellation.
- Staged-tree checks and clear reporting of commit-success/push-failure states.
- Release workflow smoke tests for the supported target binaries.

The alpha is ready for local source builds and testing. A published prerelease is
still required before release archives and the installer can be used by general
users.

Local verification on Linux (2026-09-07): 25 Rust tests and 16 Python tests passed,
along with formatting, Clippy, documentation, shell/workflow linting, and source
package verification. A live Codex smoke test using `gpt-5.6-luna` generated a
Conventional Commit and committed a synthetic file in a disposable repository
with `--yes --no-push`. macOS execution still need GitHub CI results.

## Distribution maturity

- Publish a stable release after the core workflow is exercised by users.
- Add a Homebrew tap, then consider core submission once eligible.
- Select/reserve a Cargo registry package name, then enable publishing.
- Add Linux distro packages according to demand.
- Review macOS notarization, musl binaries, and older-OS support.
- Add an ownership-aware explicit updater after installer receipts are designed.

## Later capabilities

- Additional harnesses and a versioned external-command adapter protocol.
- A dry run using an isolated index.
- Optional account selection within adapters.
- Hosted docs, generated shell completions, and man pages once the CLI stabilizes.
