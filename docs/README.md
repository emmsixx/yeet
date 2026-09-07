# Yeet documentation

The repository contains the first usable application workflow and its distribution
scaffolding. The alpha supports system Git, manual messages, the Codex generator,
and the three documented writing styles.

## Using Yeet

- [Installation and updates](installation.md): source builds, release archives,
  the installer, platform support, and uninstalling.
- [Configuration](configuration.md): settings, profiles, and styles.
- [Roadmap](roadmap.md): current status and release milestones.

## Developing and maintaining Yeet

- [Contributing](../CONTRIBUTING.md): environment, checks, and pull requests.
- [Architecture](architecture.md): domain and adapter boundaries.
- [Release process](releasing.md): CI, versioning, publication, and package managers.
- [Update design](updates.md): installation ownership and future update commands.
- [Security policy](../SECURITY.md) and [changelog](../CHANGELOG.md).

Docs live alongside code and ship inside release archives. CI checks relative file
links and builds Rust API docs. A hosted documentation site can follow when the
user-facing interface stabilizes.
