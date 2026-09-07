# Standalone Yeet architecture

Status: the first usable application behavior described here is implemented in the
Rust crate. Additional harnesses, isolated-index dry-run behavior, and distribution
publication remain deferred; see the [roadmap](roadmap.md).

Yeet owns the Git workflow. A generator supplies a commit draft; it never stages,
commits, or pushes. Manual messages bypass generation entirely. Implement the CLI
in Rust as a single Cargo package with a reusable library and a thin binary.

## Configuration and instructions

Use `$XDG_CONFIG_HOME/yeet/` when XDG_CONFIG_HOME is an absolute path; otherwise
use `~/.config/yeet/`, including on macOS. Keep settings in `config.toml` and
writing guidance in `instructions.md`.

```text
~/.config/yeet/
  config.toml
  instructions.md
```

Configuration file:

```toml
version = 1

[generation]
profile = "default"
instructions_file = "instructions.md"

[profiles.default]
harness = "codex"
model = "gpt-5.6-luna"

[profiles.default.options]
reasoning_effort = "low"

[workflow]
push = true

[commit]
style = "repository" # repository | conventional | custom

[context]
stat_max_chars = 6000
patch_max_chars = 40000
```

The model and reasoning values preserve the supplied handoff's preferences;
they are configurable strings, not a promise of availability for every account.
When config is absent, use these defaults with no additional instructions.

Use named profiles so each harness keeps its own model and options. `--profile`
selects a profile; `--model` overrides its model for one invocation. Options such
as reasoning effort belong to the adapter, not a supposedly universal model API.
Unknown harnesses or unsupported options produce actionable errors before staging.
Ship a Codex adapter first; modularity does not require shipping every integration.

Settings precedence, highest first:

1. Explicit CLI flags.
2. Environment overrides: `YEET_PROFILE`, `YEET_MODEL`, `YEET_STYLE`.
3. The selected config file (`--config PATH` or the default location).
4. Built-in defaults.

During migration, accept `YEET_CODEX_MODEL` and
`YEET_CODEX_REASONING_EFFORT` for Codex profiles only. The legacy model variable
has lower precedence than `YEET_MODEL` but higher precedence than config.
The legacy reasoning variable overrides the configured Codex reasoning option.
Do not silently fall back to another model or harness after failure.

Resolve config-relative paths against the config file's directory. Expand a
leading `~/` only; do not evaluate shell expressions or interpolate environment
variables in config values. Reject invalid TOML, unsupported config versions,
unknown keys, invalid limits, and missing selected profiles.

### Commit writing styles

Style is independent of the harness/model profile. Select it with `commit.style`,
`YEET_STYLE`, or `--style repository|conventional|custom`. Use repository conventions
by default, matching the first option in the supplied selector.

| Style | Generation behavior | Validation |
| --- | --- | --- |
| Repository conventions (`repository`) | Follow the subject/body patterns in recent local commit messages. | Universal draft checks; inferred conventions are guidance, not hard rules. |
| Conventional Commits (`conventional`) | Generate `type(scope)!: description`, with optional scope and breaking marker. | Enforce Conventional Commit syntax plus the rules below. |
| Custom instructions (`custom`) | Use the selected Markdown file and/or one-off instructions as the primary writing style. | Universal draft checks; prose instructions do not become executable validators. |

Repository mode samples up to 20 recent non-merge commits reachable from HEAD,
including subjects and bodies, bounded to 12,000 Unicode code points total.
Use local history only; never fetch. Collect it before staging in generated mode.
Treat examples as style evidence, not commands or facts about the new change.
Do not copy ticket IDs or scopes unless supported by the current change and
explicit guidance. Mixed histories guide the model toward the most common recent
patterns without inventing a mandatory repository policy.

For an unborn branch or no eligible history, use a concise imperative plain-text
subject and an optional explanatory body. Show that fallback in the preview.
An actual Git read error is an error, not evidence of empty history. The prompt
must identify any truncated examples. Document that generation in this mode also
shares the sampled commit messages with the configured harness/model service.

Custom mode requires non-whitespace instructions from a selected file or
`--instructions` in generated mode; fail before staging if none are available.
In repository and conventional modes, instructions supplement the selected style.
Explicit guidance takes precedence over inferred examples, but cannot override
hard validation rules. Selecting custom mode removes the Conventional Commit
requirement; it does not change the JSON response format or Git workflow.

CLI usage examples:

```sh
yeet --style repository
yeet --style conventional --model gpt-5.6-luna
yeet --style custom --instructions-file ~/.config/yeet/team-style.md
yeet --style custom --instructions "Start with the ticket ID PROJ-123"
yeet --style repository "Handle empty configuration files"
```

Manual messages use the selected style's mechanical validation only: they never
need history inspection, instruction loading, or a model call. The default thus
intentionally accepts plain-text manual messages; choose `conventional` to retain
the original function's Conventional Commit requirement.

### Why Markdown for instructions

Writing guidance benefits from paragraphs, examples, and version control without
TOML escaping. Keep a single persistent source: an instructions file, not both an
inline config string and a competing Markdown file.

```markdown
Prefer a scope when one package or subsystem owns the change.
Explain user-visible behavior instead of listing edited files.
Use a body only when the motivation is not clear from the subject.
```

Instruction inputs:

- Global: `generation.instructions_file`; if unset, load `instructions.md` beside
  the config when present. An explicitly configured missing file is an error.
- Per invocation: `--instructions-file PATH` replaces the global file for that
  run; CLI paths are relative to the invocation directory.
- One-off: `--instructions "Mention the migration requirement"` appends guidance.

Custom guidance affects generation, not workflow permissions, output schema, or
commit validation. Manual mode does not load instruction files or require a
generator executable. Config itself must still parse correctly.

Do not automatically load repository-local config or instructions in v1. A cloned
repository must not choose executables or silently add generation instructions.
A team can explicitly select a checked-in instructions file. Yeet's instruction
loading is separate from any ambient settings a harness reads; each adapter must
document and test how it isolates or inherits those settings.

`yeet config init` should create a starter config and instructions file without
overwriting existing files. `yeet config show` should show resolved settings and
their source paths. Neither command invokes Git or a generator. Authentication
remains with the selected harness; do not store API keys in this config.

## Workflow contract

```text
preflight → staged → drafted → approved → committed → pushed
```

1. Parse configuration and arguments, verify a Git worktree, and check dependencies.
   Only generated mode requires the selected harness. Validate a manual subject
   before staging. Reject noninteractive invocation without `--yes` before staging.
2. Run `git add -A`, preserving the existing stage-all behavior.
3. Require staged changes. Distinguish an empty diff from a Git command failure.
4. Capture the staged tree ID. In generated mode, collect repository context,
   invoke the generator, and parse its structured response.
5. Validate the draft and recheck the staged tree ID. Abort on a mismatch.
6. Display staged stats and the complete proposed message. Ask for Enter;
   Ctrl+C or EOF cancels. `--yes` explicitly skips this prompt.
7. Recheck the staged tree immediately before committing. Abort if it changed.
8. Run `git commit`, then plain `git push` unless `--no-push` or config disables it.

Use system Git, preserving normal hooks, signing, authentication, and push config.
Yeet does not fetch, pull, pick a remote, or establish an upstream. Hooks remain
authoritative and may change the final commit; tree checks catch ordinary index
changes but are not a transaction against concurrent Git processes or hooks.

No rollback: generation failures and cancellation leave changes staged; a commit
failure preserves Git's resulting state; push failure leaves a local commit.
Report partial success explicitly, including the commit ID and failed push.
Never retry a whole commit-and-push workflow automatically.

The standalone version intentionally improves on the shell function by rejecting
invalid manual input and missing dependencies before changing the index.

Defer `--dry-run` until its semantics are implemented deliberately: it should use
an isolated temporary index to preview what stage-all would commit, without
changing the real index or committing/pushing. Explain whether it invokes the
generator. Do not label a command that mutates the real index a dry run.

## Module boundaries

| Component | Responsibility |
| --- | --- |
| CLI / config resolver | Parse flags, resolve profiles, validate config, construct adapters. |
| Application | Advance the workflow using injected interfaces; no subprocess or terminal code. |
| GitGateway | Verify worktree, stage, inspect index/tree/context, commit, push, report commit ID. |
| MessageSource | Manual and generated sources returning the same `CommitDraft`. |
| StyleResolver | Select style, load applicable instructions/history, and describe any fallback. |
| PromptBuilder | Assemble schema requirements, resolved style guidance, and bounded repository context. |
| MessageGenerator | Generate a draft from a request; no Git workflow access. |
| CommitPolicy | Validate manual and generated drafts independently of the harness. |
| TerminalUI | Progress, preview, confirmation, cancellation, partial-success errors. |
| ProcessRunner | Argument-array execution, stdin, captured output, cancellation, exit status. |

Core Rust domain types and traits:

```rust
pub struct CommitDraft {
    pub subject: String,
    pub body: String,
}

pub enum CommitStyle {
    Repository,
    Conventional,
    Custom,
}

pub trait MessageGenerator {
    fn generate(&self, request: &GenerationRequest)
        -> Result<CommitDraft, GenerationError>;
}
```

`RepositoryContext` contains branch, staged tree ID, stat, patch, and truncation
metadata. `ResolvedStyle` contains the selected style, bounded history examples,
loaded instructions, and fallback information. `GenerationRequest` combines the
context, resolved style, and mechanical policy. Keep these types independent of
Codex and terminal rendering.

Adapters are constructed with their validated profile and ProcessRunner. A registry
maps harness names to adapter factories. Tests use fake generators and runners;
application code never switches on a model name or builds harness arguments.

An external-command adapter can follow later: pass a versioned JSON request over
stdin and require a JSON draft on stdout, with diagnostics on stderr. Configure
the executable and argument array in user config; never execute a shell command
string. Each adapter owns its invocation protocol and capability validation.

## Generation protocol and policy

Supply branch, staged diff stat (6,000 characters), and staged binary-capable
patch (40,000 characters), preserving the initial defaults. Disable external diff
drivers and text conversion. Bound collection as well as prompt construction,
truncate on valid text boundaries, and mark omissions explicitly. Binary patches
can consume the budget; a future context policy can substitute binary summaries.

Treat repository content as data, not instructions. State the required output
schema and policy separately from custom writing guidance and delimited context.
Document in setup/help that generated mode passes staged content to the selected
harness and its configured model service. Do not log patches by default.

Require exactly this shape, parsed natively without jq:

```json
{"subject": "feat(config): support named generation profiles", "body": ""}
```

Reject missing/extra keys, non-string fields, empty subjects, multiline subjects,
and NUL characters. Bound response size and generation duration. On cancellation,
stop the child process and clean up temporary resources.

Universal validation applies to every style and both message sources: require a
non-whitespace single-line subject, string body, and no NUL characters. Generated
output additionally follows the exact JSON schema and bounded response size.
Join positional manual arguments with spaces, as before.

For `conventional`, require a lowercase type (including custom types), optional
nonempty scope, optional `!`, then `: ` and a non-whitespace description. Enforce
at most 72 Unicode code points and no trailing period for both manual and generated
subjects. This deliberately replaces the earlier compatible/strict proposal with
one consistent Conventional Commit policy. The length and punctuation constraints
are Yeet's writing rules, not a claim that the Conventional Commits specification
requires them. Imperative wording remains prompt guidance, not a regex check.

Repository and custom styles do not impose a type prefix, lowercase text, a
72-character limit, or punctuation rules. Repository fallback guidance recommends
a concise subject but does not add a hidden validator. Preview the active style
alongside the draft so users can see which policy applies.

The initial Codex adapter should preserve the handoff's ephemeral, read-only,
non-approving, schema-constrained intent, invoke authenticated `codex` directly,
and parse its final response. Verify exact CLI flags and isolation behavior against
the supported CLI at implementation time. Prefer stdin for prompts; use temporary
files only where required, with cleanup on failure and signals. No dotfiles or
multi-account dependency. Account selection can be an adapter feature later.

## Delivery and verification

First implement config resolution, manual mode, Git orchestration, confirmation,
`--yes`, and `--no-push`. Then add PromptBuilder and the Codex adapter behind the
same workflow. Defer other harnesses, account management, automatic repo config,
and dry-run until the core contract is working.

Use focused unit tests for config precedence/path resolution, policy validation,
JSON parsing, style selection, and truncation. Cover repository style with empty
and mixed histories, custom style with missing instructions, plain-text manual
messages, and Conventional Commit validation independent of the harness. Integration tests should use temporary worktrees,
a local bare remote, and a fake generator to cover stage-all (including deleted
and untracked files), cancellation, generator failure, index changes during
generation/confirmation, hook rejection, successful commit/push, push failure,
and manual operation with no AI executable installed. Never require a paid model
call to run the suite.

## Rust layout

Start with one crate; separate modules provide the adapter boundaries without a
multi-crate workspace or runtime plugin system.

```text
Cargo.toml
src/
  main.rs                 # CLI entry point and exit reporting
  lib.rs                  # application assembly
  cli.rs
  config.rs
  application.rs
  domain/
    mod.rs
    commit.rs             # CommitDraft, CommitStyle, validation
    context.rs
  generation/
    mod.rs                # MessageGenerator trait and profile registry
    style.rs              # style resolution and history examples
    prompt.rs
  adapters/
    mod.rs
    git.rs
    codex.rs
    process.rs
  ui.rs
tests/
  workflow.rs
  config.rs
```

Use traits at external boundaries and concrete structs/enums for internal data.
Use argument-array subprocess execution with OS-native paths. A synchronous
workflow is sufficient initially; choose process supervision that can enforce
timeouts and cancellation while draining stdout/stderr without deadlocks. Parse
TOML and JSON into typed Rust values and manage temporary resources with RAII,
plus explicit child termination on cancellation. Select dependency versions at
implementation time. Run formatting, Clippy, and Cargo tests in CI.

## Distribution and maintenance

The application is packaged as a single native executable. Build, CI, versioning,
release archives, and package-manager plans are specified in the
[release guide](releasing.md). The [update design](updates.md) keeps install
ownership and executable updates separate from Git orchestration.
