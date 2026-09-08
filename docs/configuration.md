# Configuration

Run `yeet config init`, then edit `~/.config/yeet/config.toml`.
An absolute `$XDG_CONFIG_HOME` replaces `~/.config`. Use `yeet config show`
to see the resolved settings.

```toml
version = 1

[profiles.default]
harness = "codex"
model = "gpt-5.6-luna"

[commit]
style = "repository"
```

## Writing style

- `repository` (default): follow recent commit messages.
- `conventional`: require `type(scope): description`, with optional scope or `!`,
  lowercase type, at most 72 characters, and no trailing period.
- `custom`: follow your instructions. Generated mode requires some guidance.

Put persistent guidance in `instructions.md` beside the config, or supply it per run:

```sh
yeet --style conventional
yeet --instructions "Mention ticket PROJ-123"
yeet --style custom --instructions-file ./team-style.md
```

`--instructions-file` replaces the default file; `--instructions` adds guidance.
Manual messages skip instruction loading and generation.

## Models and profiles

Codex is the currently supported harness. It handles authentication.
Add named profiles under `[profiles.NAME]` and choose one with `--profile NAME`.
Use `--model MODEL` for a one-off model override. Set reasoning effort under
`[profiles.NAME.options]` with `reasoning_effort = "low"`.

Flags override `YEET_PROFILE`, `YEET_MODEL`, and `YEET_STYLE`, then config, then
built-in defaults. Legacy `YEET_CODEX_MODEL` and `YEET_CODEX_REASONING_EFFORT`
are also accepted for Codex; `YEET_MODEL` takes priority over the legacy model.

`--config PATH` selects another config. Paths inside it are relative to that file;
CLI paths are relative to your working directory. Repository-local config is
not loaded automatically.

See the [full config example](../examples/config.toml) for instruction-file,
push, and context-limit settings.
