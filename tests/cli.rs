// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::Command;

fn yeet() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yeet"))
}

#[test]
fn version_matches_package_metadata() {
    let output = yeet().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("yeet {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_is_available_without_git_or_a_generator() {
    let output = yeet().env("PATH", "").arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("Stage changes"));
    assert!(help.contains("--no-push"));
}

#[test]
fn noninteractive_workflow_requires_explicit_approval() {
    let output = yeet().output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not interactive")
    );
}

#[test]
fn unsupported_arguments_are_usage_errors() {
    let output = yeet().arg("--not-a-real-option").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[cfg(windows)]
#[test]
fn windows_config_uses_userprofile_without_home() {
    let root = tempfile::tempdir().unwrap();
    let output = yeet()
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env("USERPROFILE", root.path())
        .args(["config", "show"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let expected = root.path().join(".config/yeet/config.toml");
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains(&expected.display().to_string())
    );
}
