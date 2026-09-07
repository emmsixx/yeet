# Configuration

This guide describes the configuration loader and override behavior in the
current alpha release.

Use `$XDG_CONFIG_HOME/yeet/config.toml` when XDG_CONFIG_HOME is absolute, otherwise
`~/.config/yeet/config.toml`. Writing guidance belongs in `instructions.md` next
to the config. Start with the [complete config example](../examples/config.toml)
and [Markdown example](../examples/instructions.md).

On Windows, `~` uses `HOME` when set and otherwise `USERPROFILE`.

## Harness and model

A named profile selects a harness, model, and adapter-specific options:

```toml
[generation]
profile = "default"
instructions_file = "instructions.md"

[profiles.default]
harness = "codex"
model = "gpt-5.6-luna"

[profiles.default.options]
reasoning_effort = "low"
```

The current adapter invokes authenticated `codex exec` directly. The chosen model
is a configurable preference, not a guarantee of account access. Credentials
remain with the harness. Additional adapters can reuse the same application
workflow.

Generated runs pass the prompt over standard input and request the final message
through `--output-last-message` and `--output-schema`. Yeet supplies
`--ephemeral`, `--sandbox read-only`, `-c approval_policy="never"`, and
`--ignore-rules`; it does not grant the harness write access to the repository
or store authentication credentials.

## Writing style

```toml
[commit]
style = "repository"
```

- `repository`: use recent local commit messages as style examples; fall back to
  concise plain-text subjects when there is no eligible history.
- `conventional`: require Conventional Commit syntax and Yeet's lowercase,
  72-character, and no-trailing-period rules.
- `custom`: use your Markdown file and/or one-off instructions as the primary
  style. Generated mode requires nonempty instructions.

`--style` overrides the setting for a run. `--instructions-file PATH` selects a
different file; `--instructions "..."` adds one-off guidance. Repository and custom
styles do not require Conventional Commit prefixes. Manual messages bypass the
model, history sampling, and instruction loading.

## Resolution and validation

CLI flags override `YEET_PROFILE`, `YEET_MODEL`, and `YEET_STYLE`, which override
config and then defaults. `--config PATH` selects an explicit config file.
Config-relative paths resolve beside that config; CLI paths resolve from the
invocation directory. Unknown keys and invalid settings fail before staging.

There is no automatic repository-local config loading in this release.
Custom instructions cannot authorize extra tools or override workflow confirmation.
The [architecture](architecture.md) specifies full precedence, legacy environment
compatibility, history limits, and validation behavior.
