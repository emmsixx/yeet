# Contributing to Yeet

Yeet is in early development. The [architecture](docs/architecture.md) documents
the implemented workflow and its boundaries; the [roadmap](docs/roadmap.md)
distinguishes completed work from deferred capabilities. Discuss substantial
workflow or distribution changes in an issue before building them.

## Local setup

Install rustup and your platform's native linker. Rustup reads the pinned
`rust-toolchain.toml`; the same version is tested in CI. Python 3.11+ is needed
for packaging and documentation checks. ShellCheck is needed for the installer.

```sh
cargo build --locked
cargo run -- --help
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc
cargo doc --locked --no-deps --all-features
python3 scripts/check_docs.py
python3 -m unittest discover -s tests -p 'test_*.py' -v
shellcheck install.sh
cargo package --locked
```

Run `cargo fmt --all` to apply formatting. `cargo package` requires a clean working
tree; `--allow-dirty` is useful for a local preview only. No tests should call a
paid model service, push to a real remote, or change your personal configuration.
Installer tests use local fixtures and a fake downloader.

For workflow changes, also run `actionlint` (CI pins v1.7.12). It checks workflow
syntax, expressions, action inputs, and embedded shell scripts.

## Code organization

Keep one Cargo package with a thin executable and reusable library. Add the modules
in the architecture as behavior is implemented, rather than checking in empty
abstraction layers. Put subprocess, terminal, and generator integration behind
traits; keep workflow and validation logic testable without them.

Use native paths and argument arrays for subprocesses. Avoid shell evaluation,
panics in user-facing error paths, and leaking patches or credentials into logs.
Changes to staging, confirmation, hooks, or push handling need integration tests
using disposable repositories. Failure messages must say when a commit already
exists locally.

Commit `Cargo.lock` because this is an application. Add dependencies deliberately;
review their maintenance, license compatibility, and impact on supported targets.
Do not add Tokio or a workspace until there is a concrete need. Toolchain changes
must update `Cargo.toml` and `rust-toolchain.toml` together.

## Pull requests

Explain the problem and resulting behavior, list relevant checks, and document
known limitations. Update user docs and `CHANGELOG.md` for user-visible changes.
Use descriptive commits; contributions do not require Conventional Commits.
Keep unrelated formatting and refactors separate.

CI runs formatting, Clippy, Rust tests on Linux/macOS, current-stable
compatibility tests, documentation builds, local documentation links, source
packaging, and distribution tests. Release builds additionally test every shipped
target. Maintainers should require the `CI passed` check before merging.

Changes to workflows and `install.sh` deserve careful review because they deliver
executables to users. Never publish directly from a pull request. See the
[release guide](docs/releasing.md) for the maintainer workflow.

## Community and licensing

Be respectful, assume good intent, and focus feedback on the work. Harassment and
disclosure of other people's private information are not acceptable. Maintainers
may moderate discussions to keep the project welcoming and productive.

Unless explicitly stated otherwise, contributions are made under Yeet's
GPL-3.0-or-later license. See [LICENSE](LICENSE). No contributor license agreement
is required.
