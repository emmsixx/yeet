# Changelog

All user-visible changes are recorded here. Release tags use `vMAJOR.MINOR.PATCH`,
optionally followed by a prerelease suffix. No release has been published yet.

## Unreleased

### Added

- First usable Rust workflow with stage-all, preview, confirmation, commit, and
  optional push through system Git.
- Typed configuration, named generation profiles, Markdown instructions, and
  three commit writing styles.
- Codex `exec` adapter with bounded context, exact JSON output validation,
  read-only ephemeral invocation, timeout, cancellation, and temporary-file cleanup.
- Integration coverage for temporary repositories, local bare remotes, fake
  generators, cancellation, and partial push success.
- CI for formatting, linting, tests, docs, and packaging.
- Tagged-release workflow producing platform archives, checksums, build provenance,
  corresponding project source, and a draft GitHub Release.
- Linux/macOS installer with checksum verification and atomic binary replacement.
- Contributor, installation, configuration, architecture, release, and security docs.

Additional harnesses, isolated-index dry-run behavior, account selection,
background updating, and package-manager publication remain deferred.
