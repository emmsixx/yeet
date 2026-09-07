// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;

use yeet_cli::config::{
    DEFAULT_PATCH_MAX_CHARS, DEFAULT_PROFILE, DEFAULT_REASONING_EFFORT, DEFAULT_STAT_MAX_CHARS,
    init, resolve, show,
};
use yeet_cli::domain::{CommitStyle, parse_generated_draft};
use yeet_cli::generation::{HistoryEvidence, resolve_generated_style};

#[test]
fn missing_config_uses_documented_defaults() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("missing.toml");
    let resolved = resolve(Some(&config), None, None, None, root.path()).unwrap();
    assert_eq!(resolved.profile.name, DEFAULT_PROFILE);
    assert_eq!(resolved.profile.model, "gpt-5.6-luna");
    assert_eq!(resolved.profile.reasoning_effort, DEFAULT_REASONING_EFFORT);
    assert_eq!(resolved.limits.stat_max_chars, DEFAULT_STAT_MAX_CHARS);
    assert_eq!(resolved.limits.patch_max_chars, DEFAULT_PATCH_MAX_CHARS);
    assert!(resolved.instructions_file.is_none());
}

#[test]
fn config_init_is_idempotent_and_show_reports_sources() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("settings/config.toml");
    let created = init(Some(&config), root.path()).unwrap();
    assert!(created.iter().all(|message| message.contains("created")));
    let original_config = fs::read_to_string(&config).unwrap();
    let original_instructions =
        fs::read_to_string(config.parent().unwrap().join("instructions.md")).unwrap();

    let kept = init(Some(&config), root.path()).unwrap();
    assert!(kept.iter().all(|message| message.contains("kept")));
    assert_eq!(fs::read_to_string(&config).unwrap(), original_config);
    assert_eq!(
        fs::read_to_string(config.parent().unwrap().join("instructions.md")).unwrap(),
        original_instructions
    );

    let resolved = resolve(Some(&config), None, None, None, root.path()).unwrap();
    let output = show(&resolved);
    assert!(output.contains("(loaded)"));
    assert!(output.contains("instructions.md (loaded)"));
}

#[test]
fn config_paths_and_overrides_resolve_correctly() {
    let root = tempfile::tempdir().unwrap();
    let config_dir = root.path().join("settings");
    fs::create_dir_all(&config_dir).unwrap();
    let config = config_dir.join("config.toml");
    fs::write(
        &config,
        r#"
version = 1
[generation]
profile = "work"
instructions_file = "guide.md"
[profiles.work]
harness = "codex"
model = "configured-model"
[profiles.work.options]
reasoning_effort = "medium"
[commit]
style = "custom"
[workflow]
push = false
"#,
    )
    .unwrap();
    fs::write(config_dir.join("guide.md"), "Use a body.").unwrap();

    let resolved = resolve(
        Some(&config),
        Some("work"),
        Some("one-shot-model"),
        Some("repository"),
        root.path(),
    )
    .unwrap();
    assert_eq!(resolved.profile.model, "one-shot-model");
    assert_eq!(resolved.profile.reasoning_effort, "medium");
    assert_eq!(resolved.style.as_str(), "repository");
    assert!(!resolved.push);
    assert_eq!(
        resolved.instructions_file,
        Some(config_dir.join("guide.md"))
    );
}

#[test]
fn unknown_config_keys_and_invalid_limits_fail() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    fs::write(
        &config,
        r#"
version = 1
profiles = {}
unexpected = true
"#,
    )
    .unwrap();
    let error = resolve(Some(&config), None, None, None, root.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("invalid TOML"));

    fs::write(
        &config,
        r#"
version = 1
[profiles.default]
harness = "codex"
model = "model"
[context]
stat_max_chars = 0
"#,
    )
    .unwrap();
    let error = resolve(Some(&config), None, None, None, root.path())
        .unwrap_err()
        .to_string();
    assert!(error.contains("stat_max_chars"));
}

#[test]
fn explicitly_empty_overrides_fail_instead_of_falling_back() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    fs::write(
        &config,
        r#"
version = 1
[profiles.default]
harness = "codex"
model = "configured-model"
"#,
    )
    .unwrap();

    assert!(
        resolve(Some(&config), Some(" "), None, None, root.path())
            .unwrap_err()
            .to_string()
            .contains("generation profile")
    );
    assert!(
        resolve(Some(&config), None, Some("\t"), None, root.path())
            .unwrap_err()
            .to_string()
            .contains("model")
    );
    assert!(
        resolve(Some(&config), None, None, Some(""), root.path())
            .unwrap_err()
            .to_string()
            .contains("commit style")
    );
}

#[test]
fn generated_json_requires_exact_two_string_keys() {
    let valid = parse_generated_draft(
        br#"{"subject":"feat: add parser","body":""}"#,
        CommitStyle::Repository,
        4096,
    )
    .unwrap();
    assert_eq!(valid.subject, "feat: add parser");

    for response in [
        br#"{"subject":"ok"}"#.as_slice(),
        br#"{"subject":"ok","body":"","extra":"no"}"#.as_slice(),
        br#"{"subject":"ok","body":"","body":"duplicate"}"#.as_slice(),
        br#"{"subject":"ok","body":""} trailing"#.as_slice(),
    ] {
        assert!(parse_generated_draft(response, CommitStyle::Repository, 4096).is_err());
    }
}

#[test]
fn custom_generation_requires_instructions_and_marks_history_fallback() {
    let missing = resolve_generated_style(
        CommitStyle::Custom,
        Some(std::path::Path::new("/tmp/yeet-no-such-instructions.md")),
        None,
        None,
        None,
    )
    .unwrap_err()
    .to_string();
    assert!(missing.contains("does not exist"));

    let style = resolve_generated_style(
        CommitStyle::Repository,
        None,
        None,
        None,
        Some(HistoryEvidence {
            examples: "No eligible history.".to_string(),
            truncated: false,
            fallback: true,
        }),
    )
    .unwrap();
    assert!(style.fallback.is_some());
}
