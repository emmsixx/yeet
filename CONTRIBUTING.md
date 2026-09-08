# Contributing

Small, focused PRs welcome. Describe what changed and how you checked it.
Discuss larger workflow changes in an issue first.

## Setup and checks

Install Rust, a native linker, Python 3.11+, and ShellCheck. The repo pins its
Rust toolchain. Build with `cargo build --locked`.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc
cargo doc --locked --no-deps --all-features
python3 scripts/check_docs.py
python3 -m unittest discover -s tests -p 'test_*.py'
shellcheck install.sh
cargo package --locked
```

Use `cargo fmt --all` to format and `cargo package --allow-dirty --locked` for
an uncommitted packaging preview. Run `actionlint` for workflow changes.
CI tests macOS and Linux.

## Keep it simple

- Keep Git, generation, and terminal I/O behind the existing interfaces.
- Test workflow changes with disposable repositories and fake generators;
  tests must not call paid models, push to real remotes, or change personal config.
- Commit `Cargo.lock`; update both toolchain files when changing Rust versions.
- Update docs and the changelog when behavior changes.

See [architecture](docs/architecture.md) and [releasing](docs/releasing.md).
Be respectful. Contributions use [GPL-3.0-or-later](LICENSE).
