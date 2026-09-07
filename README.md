# Yeet

A Rust CLI for macOS and Linux that stages changes, drafts a commit message in
your preferred style, and commits and pushes after review.

**Status: first usable alpha.** Yeet stages all changes, validates a manual or
generated commit message, previews it, commits after approval, and pushes through
system Git. The alpha ships a Codex adapter, typed configuration, and three
writing styles. No public release or package manager listing is implied.

## Intended workflow

```text
stage all → draft or use your message → preview → confirm → commit → push
```

Choose a writing style independently of your harness and model:

| Style | Behavior |
| --- | --- |
| Repository conventions | Follow recent commit history; the default. |
| Conventional Commits | Use and validate `type(scope): description`. |
| Custom instructions | Follow your Markdown writing guidance. |

The Codex adapter invokes the authenticated CLI with a configurable model. Manual
messages work without an AI harness. Yeet uses system Git so hooks, signing, and
credential helpers continue to work.

## Build from source

Install [Rust through rustup](https://www.rust-lang.org/tools/install) and a native
C/C++ linker for your platform, then:

```sh
git clone https://github.com/emmsixx/yeet.git
cd yeet
cargo build --locked
cargo run -- --help
cargo test --locked
```

The repo pins Rust 1.94.1, currently also the minimum supported version. To install
your local build:

```sh
cargo install --path . --locked
yeet --version
```

Manual messages work without an AI executable:

```sh
yeet --yes --no-push "Explain the staged change"
yeet --style conventional --yes --no-push "feat: explain the staged change"
```

With no positional message, Yeet invokes the configured authenticated Codex CLI
in an ephemeral read-only session. Review the proposed message interactively, or
use `--yes` in automation. `--no-push` leaves a successful local commit in place.
Generation sends bounded staged statistics, patch content, selected style
guidance, and (in repository style) sampled local commit messages to the
configured harness/model service.

Release archives and a checksum-verifying installer are prepared for Linux and
macOS on x86_64/ARM64. They become usable after a release is
published. See [installation and updates](docs/installation.md). Homebrew
and crates.io distribution are planned, not available channels.

## Configuration

Settings live in `$XDG_CONFIG_HOME/yeet/config.toml`, falling back to
`~/.config/yeet/config.toml`. Markdown instructions live beside the config.

```toml
version = 1

[generation]
profile = "default"
instructions_file = "instructions.md"

[profiles.default]
harness = "codex"
model = "gpt-5.6-luna"

[commit]
style = "repository"
```

Use `yeet config init` to create starter files without overwriting existing files.
See the [complete example](examples/config.toml), [instructions example](examples/instructions.md),
and [configuration guide](docs/configuration.md).

## Development and documentation

- [Contributing](CONTRIBUTING.md): setup, checks, and pull requests.
- [Documentation index](docs/README.md): user and maintainer guides.
- [Architecture](docs/architecture.md): workflow and module boundaries.
- [Release process](docs/releasing.md): versioning, artifacts, and publication.
- [Roadmap](docs/roadmap.md): implemented milestones and deferred capabilities.
- [Changelog](CHANGELOG.md) and [security policy](SECURITY.md).

Bug reports should include the version, OS, install method, and reproduction steps.
Please remove credentials and private repository content from reports.

## License

Yeet is licensed under the GNU General Public License, version 3 or (at your
option) any later version: **GPL-3.0-or-later**. See [LICENSE](LICENSE).
