# Architecture

One Rust crate, a thin CLI, and a reusable library. Yeet controls Git;
the message generator only returns a subject and body.

## Workflow

1. Validate config, dependencies, repository, and any manual message.
2. Stage everything with `git add -A`; stop if there are no changes.
3. Use the supplied message or generate one from the staged changes.
4. Validate and preview the message, then wait for Enter (unless `--yes`).
5. Commit, then run plain `git push` unless disabled by `--no-push` or config.

Staged-tree checks after generation and before committing catch index changes.
Git hooks, signing, and credentials remain under system Git's control. Yeet does
not fetch, pull, select remotes, or set upstreams.

Cancellation and generation failure leave changes staged. Push failure leaves
the local commit and reports its ID. There is no automatic rollback or retry.

## Code map

| Module | Owns |
| --- | --- |
| `cli.rs`, `config.rs`, `lib.rs` | Arguments, settings, adapter construction |
| `application.rs` | Workflow and state transitions |
| `domain/` | Drafts, repository context, validation |
| `generation/` | Generator interface, style resolution, prompts |
| `adapters/git.rs` | System Git operations |
| `adapters/codex.rs` | Codex invocation and response parsing |
| `adapters/process.rs` | Subprocess I/O, timeouts, cancellation |
| `ui.rs` | Preview, confirmation, progress |

Use argument arrays, not shell strings. Add harnesses behind `MessageGenerator`;
keep model-specific behavior out of the workflow.

## Generation

The prompt includes the branch, staged stat (6,000 characters by default), patch (40,000),
and writing guidance. Repository style also samples up to 20 non-merge commits,
bounded to 12,000 characters. Empty history falls back to concise plain text.
Repository content is data, not instructions.

Codex runs ephemeral, read-only, with approvals disabled and `--ignore-rules`.
Prompts go through stdin; temporary files hold the schema and final response.
Authentication stays with Codex. Generation times out after 120 seconds.

The response must contain exactly two strings:

```json
{"subject": "Fix the login redirect", "body": ""}
```

Responses are limited to 128 KiB. All drafts require a nonempty, single-line
subject and no NUL characters. Conventional style adds the rules in
[configuration](configuration.md). Temporary resources are cleaned up on failure.

Tests cover config, validation, generation, Git operations, cancellation,
concurrent index changes, and push failures using fakes and temporary repositories.
