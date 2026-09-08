# Yeet

Commit and push in one command. For macOS and Linux.

```sh
yeet                         # Generate a commit message
yeet "Fix the login redirect" # Use your own
```

Yeet stages **all changes**, shows the message, and waits for Enter before
committing and pushing. Generated messages use your authenticated Codex CLI;
your own messages need only Git.

## Install

```sh
brew install emmsixx/tap/yeet
```

Or use the [installer or release archives](docs/installation.md)—no Rust needed.
[First alpha](https://github.com/emmsixx/yeet/releases/tag/v0.1.0-alpha.1):
feedback and [bug reports](https://github.com/emmsixx/yeet/issues) welcome.

## Options

```sh
yeet -y                      # Skip confirmation
yeet --no-push                # Commit locally
yeet --style conventional     # feat: ..., fix: ..., etc.
yeet --instructions "Keep it short"
```

By default, messages follow your recent commit style. Generation sends staged
changes and sampled commit history to the configured model.

## Configure

```sh
yeet config init
```

Edit `~/.config/yeet/config.toml` to choose a model or writing style, and
`instructions.md` beside it for writing guidance. The default is Codex with
`gpt-5.6-luna`.

[Configuration](docs/configuration.md) · [Installation](docs/installation.md) ·
[Contributing](CONTRIBUTING.md) · [Docs](docs/README.md)

[GPL-3.0-or-later](LICENSE)
