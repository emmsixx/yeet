# Update design

Status: installer reruns are implemented; an in-app updater is not.

The initial update mechanism is explicit: script users rerun the installer for a
chosen version, archive users replace their binary, and package-manager users
update through their package manager. Normal `yeet` invocations never make an
update request or alter the running executable.

## Future CLI

Consider `yeet update --check` for an explicit version check and `yeet update` for
an explicit update. Keep the updater separate from Git orchestration and make it
optional at build time so package maintainers can disable it.

Before shipping this, implement:

- An install receipt recording script ownership, installed version, and target.
  Treat missing or ambiguous ownership as externally managed and print manual
  instructions. A receipt must match the executable being replaced.
- A stable default channel and an explicit prerelease channel, with proper SemVer
  ordering, platform selection, and minimum-OS checks.
- Verified release provenance plus checksums, bounded downloads, timeouts, and
  clear offline/rate-limit behavior.
- A download-to-temp, verify, and atomic replacement sequence. Keep the old binary
  on any failure and serialize competing updater processes.
- Tests for interruption, wrong platform/version, tampered downloads, read-only
  locations, package-manager ownership, and rollback.

Do not self-update Homebrew, Cargo, distro, or unknown installations.
Do not infer ownership solely from a writable directory or executable filename.

If background version notifications are added, make them opt-in, cache checks,
and skip network access in CI/noninteractive runs. Silent automatic binary
replacement is outside the current design. Updates must not rewrite user config;
future config migrations need an explicit version and recoverable backup.
