// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use std::thread;
use std::time::Duration;

use yeet_cli::adapters::CodexAdapter;
#[cfg(unix)]
use yeet_cli::adapters::SystemProcessRunner;
use yeet_cli::adapters::{
    CancellationToken, GitGateway, ProcessError, ProcessOutput, ProcessRunner, SystemGit,
};
use yeet_cli::application::{WorkflowOptions, WorkflowOutcome, WorkflowUi, run_workflow};
use yeet_cli::config::ContextLimits;
use yeet_cli::config::Profile;
use yeet_cli::domain::{CommitDraft, CommitStyle, RepositoryContext};
use yeet_cli::generation::{GenerationError, GenerationRequest, MessageGenerator};
use yeet_cli::generation::{HistoryEvidence, ResolvedStyle};

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_status(dir: &Path, args: &[&str]) -> std::process::ExitStatus {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .unwrap()
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.name", "Yeet Test"]);
    git(dir.path(), &["config", "user.email", "yeet@example.test"]);
    fs::write(dir.path().join("tracked.txt"), "initial\n").unwrap();
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-qm", "chore: initial"]);
    dir
}

#[derive(Default)]
struct FakeUi {
    confirm: bool,
    previews: Vec<String>,
    committed: Option<(String, bool)>,
    cancelled: bool,
}

impl WorkflowUi for FakeUi {
    fn progress(&mut self, _message: &str) {}

    fn preview(
        &mut self,
        _style: CommitStyle,
        _fallback: Option<&str>,
        stat: &str,
        draft: &CommitDraft,
    ) {
        self.previews
            .push(format!("{stat}\n{}", draft.full_message()));
    }

    fn confirm(&mut self) -> Result<bool, String> {
        Ok(self.confirm)
    }

    fn committed(&mut self, id: &str, pushed: bool) {
        self.committed = Some((id.to_string(), pushed));
    }

    fn cancelled(&mut self) {
        self.cancelled = true;
    }
}

struct FakeGenerator {
    draft: CommitDraft,
}

impl MessageGenerator for FakeGenerator {
    fn generate(&self, _request: &GenerationRequest) -> Result<CommitDraft, GenerationError> {
        Ok(self.draft.clone())
    }
}

struct FailingGenerator;

impl MessageGenerator for FailingGenerator {
    fn generate(&self, _request: &GenerationRequest) -> Result<CommitDraft, GenerationError> {
        Err(GenerationError::Process(
            "fake generator failed".to_string(),
        ))
    }
}

struct MutatingGenerator {
    repo: std::path::PathBuf,
}

impl MessageGenerator for MutatingGenerator {
    fn generate(&self, _request: &GenerationRequest) -> Result<CommitDraft, GenerationError> {
        fs::write(self.repo.join("tracked.txt"), "changed during generation\n").unwrap();
        git(&self.repo, &["add", "-A"]);
        Ok(CommitDraft::new("feat: generated message", ""))
    }
}

#[derive(Clone, Default)]
struct RecordingRunner {
    args: Arc<Mutex<Vec<String>>>,
    prompt: Arc<Mutex<Vec<u8>>>,
}

fn successful_exit_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}

impl ProcessRunner for RecordingRunner {
    fn run(
        &self,
        _executable: &std::ffi::OsStr,
        args: &[std::ffi::OsString],
        stdin: &[u8],
        _timeout: Duration,
        _cancellation: &CancellationToken,
    ) -> Result<ProcessOutput, ProcessError> {
        *self.args.lock().unwrap() = args
            .iter()
            .map(|value| value.to_string_lossy().to_string())
            .collect();
        *self.prompt.lock().unwrap() = stdin.to_vec();
        let output = args
            .windows(2)
            .find(|pair| pair[0] == "--output-last-message")
            .map(|pair| pair[1].clone())
            .unwrap();
        fs::write(output, r#"{"subject":"feat: generated safely","body":""}"#).unwrap();
        Ok(ProcessOutput {
            status: successful_exit_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn check_available(&self, _executable: &std::ffi::OsStr) -> Result<(), ProcessError> {
        Ok(())
    }
}

fn manual_options(message: &str, push: bool) -> WorkflowOptions {
    WorkflowOptions {
        manual_message: Some(message.to_string()),
        style: CommitStyle::Repository,
        global_instructions_file: None,
        cli_instructions_file: None,
        one_off_instructions: None,
        limits: ContextLimits {
            stat_max_chars: 6000,
            patch_max_chars: 40000,
        },
        push,
        yes: true,
        interactive: false,
    }
}

#[test]
fn manual_workflow_stages_untracked_and_commits_without_push() {
    let repo = repository();
    fs::write(repo.path().join("new.txt"), "new\n").unwrap();
    fs::remove_file(repo.path().join("tracked.txt")).unwrap();
    let git_gateway = SystemGit::new(repo.path());
    let mut ui = FakeUi::default();

    let result = run_workflow(
        &git_gateway,
        None::<&FakeGenerator>,
        &manual_options("remove tracked file", false),
        &mut ui,
    )
    .unwrap();
    let WorkflowOutcome::Committed { commit_id, pushed } = result else {
        panic!("expected commit");
    };
    assert!(!pushed);
    assert_eq!(commit_id, git(repo.path(), &["rev-parse", "HEAD"]));
    assert!(
        git(repo.path(), &["show", "--format=%s", "--no-patch", "HEAD"])
            .contains("remove tracked file")
    );
    assert!(git(repo.path(), &["status", "--porcelain"]).is_empty());
}

#[test]
fn generated_repository_style_uses_local_history_and_fake_generator() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "changed\n").unwrap();
    let git_gateway = SystemGit::new(repo.path());
    let generator = FakeGenerator {
        draft: CommitDraft::new("feat: explain staged change", "The body gives context."),
    };
    let options = WorkflowOptions {
        manual_message: None,
        style: CommitStyle::Repository,
        global_instructions_file: None,
        cli_instructions_file: None,
        one_off_instructions: None,
        limits: ContextLimits {
            stat_max_chars: 6000,
            patch_max_chars: 40000,
        },
        push: false,
        yes: true,
        interactive: false,
    };
    let mut ui = FakeUi::default();
    let result = run_workflow(&git_gateway, Some(&generator), &options, &mut ui).unwrap();
    assert!(matches!(
        result,
        WorkflowOutcome::Committed { pushed: false, .. }
    ));
    assert!(ui.previews[0].contains("feat: explain staged change"));
}

#[test]
fn repository_context_respects_unicode_limits_and_marks_truncation() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "é".repeat(500)).unwrap();
    let gateway = SystemGit::new(repo.path());
    gateway.stage_all().unwrap();
    let context = gateway
        .repository_context(&ContextLimits {
            stat_max_chars: 40,
            patch_max_chars: 120,
        })
        .unwrap();
    assert!(context.stat.chars().count() <= 40);
    assert!(context.patch.chars().count() <= 120);
    assert!(context.stat_truncation.truncated || context.patch_truncation.truncated);
    assert!(context.patch.contains("context truncated"));
}

#[test]
fn generator_failure_leaves_stage_all_changes_in_the_index() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "changed\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let mut ui = FakeUi::default();
    let options = WorkflowOptions {
        manual_message: None,
        style: CommitStyle::Repository,
        global_instructions_file: None,
        cli_instructions_file: None,
        one_off_instructions: None,
        limits: ContextLimits {
            stat_max_chars: 6000,
            patch_max_chars: 40000,
        },
        push: false,
        yes: true,
        interactive: false,
    };
    let error = run_workflow(&gateway, Some(&FailingGenerator), &options, &mut ui)
        .unwrap_err()
        .to_string();
    assert!(error.contains("fake generator failed"));
    assert_eq!(
        git_status(repo.path(), &["diff", "--cached", "--quiet"]).code(),
        Some(1)
    );
}

#[test]
fn index_change_during_generation_aborts_before_commit() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "changed\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let generator = MutatingGenerator {
        repo: repo.path().to_path_buf(),
    };
    let options = WorkflowOptions {
        manual_message: None,
        style: CommitStyle::Repository,
        global_instructions_file: None,
        cli_instructions_file: None,
        one_off_instructions: None,
        limits: ContextLimits {
            stat_max_chars: 6000,
            patch_max_chars: 40000,
        },
        push: false,
        yes: true,
        interactive: false,
    };
    let mut ui = FakeUi::default();
    let error = run_workflow(&gateway, Some(&generator), &options, &mut ui)
        .unwrap_err()
        .to_string();
    assert!(error.contains("staged tree changed"));
    assert_eq!(
        git(repo.path(), &["log", "-1", "--format=%s"]),
        "chore: initial"
    );
}

#[test]
fn cancellation_leaves_changes_staged() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "changed\n").unwrap();
    let git_gateway = SystemGit::new(repo.path());
    let mut options = manual_options("cancel this", false);
    options.yes = false;
    options.interactive = true;
    let mut ui = FakeUi {
        confirm: false,
        ..FakeUi::default()
    };
    let result = run_workflow(&git_gateway, None::<&FakeGenerator>, &options, &mut ui).unwrap();
    assert_eq!(result, WorkflowOutcome::Cancelled);
    assert!(ui.cancelled);
    assert_eq!(
        git_status(repo.path(), &["diff", "--cached", "--quiet"]).code(),
        Some(1)
    );
}

#[test]
fn conventional_policy_rejects_bad_manual_subject_before_staging() {
    let repo = repository();
    fs::write(repo.path().join("tracked.txt"), "changed\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let mut options = manual_options("Bad subject.", false);
    options.style = CommitStyle::Conventional;
    let mut ui = FakeUi::default();
    let error = run_workflow(&gateway, None::<&FakeGenerator>, &options, &mut ui)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Conventional"));
    assert_eq!(
        git_status(repo.path(), &["diff", "--cached", "--quiet"]).code(),
        Some(0)
    );
}

#[test]
fn local_bare_remote_pushes_successfully() {
    let repo = repository();
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--bare", "-q"]);
    git(
        repo.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(repo.path(), &["push", "-q", "-u", "origin", "HEAD"]);
    fs::write(repo.path().join("tracked.txt"), "pushed\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let mut ui = FakeUi::default();
    let result = run_workflow(
        &gateway,
        None::<&FakeGenerator>,
        &manual_options("push staged change", true),
        &mut ui,
    )
    .unwrap();
    assert!(matches!(
        result,
        WorkflowOutcome::Committed { pushed: true, .. }
    ));
    assert_eq!(
        git(remote.path(), &["rev-parse", "HEAD"]),
        git(repo.path(), &["rev-parse", "HEAD"])
    );
}

#[test]
fn push_failure_reports_local_commit_id() {
    let repo = repository();
    git(
        repo.path(),
        &["remote", "add", "origin", "/tmp/yeet-no-such-remote"],
    );
    fs::write(repo.path().join("tracked.txt"), "local\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let mut ui = FakeUi::default();
    let error = run_workflow(
        &gateway,
        None::<&FakeGenerator>,
        &manual_options("keep local commit", true),
        &mut ui,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("succeeded locally"));
    assert_eq!(
        git(repo.path(), &["show", "--format=%s", "--no-patch", "HEAD"]),
        "keep local commit"
    );
}

#[cfg(unix)]
#[test]
fn rejected_commit_hook_preserves_git_state() {
    use std::os::unix::fs::PermissionsExt;

    let repo = repository();
    let hook = repo.path().join(".git/hooks/pre-commit");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(repo.path().join("tracked.txt"), "hook rejects\n").unwrap();
    let gateway = SystemGit::new(repo.path());
    let mut ui = FakeUi::default();
    let error = run_workflow(
        &gateway,
        None::<&FakeGenerator>,
        &manual_options("hook rejected", false),
        &mut ui,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("git commit failed"));
    assert_eq!(
        git(repo.path(), &["log", "-1", "--format=%s"]),
        "chore: initial"
    );
    assert_eq!(
        git_status(repo.path(), &["diff", "--cached", "--quiet"]).code(),
        Some(1)
    );
}

#[cfg(unix)]
fn sleeping_command() -> (std::ffi::OsString, Vec<std::ffi::OsString>) {
    #[cfg(unix)]
    {
        (
            std::ffi::OsString::from("sh"),
            vec![
                std::ffi::OsString::from("-c"),
                std::ffi::OsString::from("sleep 5"),
            ],
        )
    }
}

#[cfg(unix)]
#[test]
fn process_runner_enforces_timeout_and_cancellation() {
    let runner = SystemProcessRunner;
    let (timed_executable, timed_args) = sleeping_command();
    let token = CancellationToken::new();
    let timed = runner.run(
        timed_executable.as_os_str(),
        &timed_args,
        &[],
        Duration::from_millis(60),
        &token,
    );
    assert!(matches!(
        timed,
        Err(yeet_cli::adapters::ProcessError::TimedOut(_))
    ));

    let token = CancellationToken::new();
    let cancel = token.clone();
    let (cancel_executable, cancel_args) = sleeping_command();
    let handle = thread::spawn(move || {
        runner.run(
            cancel_executable.as_os_str(),
            &cancel_args,
            &[],
            Duration::from_secs(5),
            &cancel,
        )
    });
    thread::sleep(Duration::from_millis(60));
    token.cancel();
    let cancelled = handle.join().unwrap();
    assert!(matches!(
        cancelled,
        Err(yeet_cli::adapters::ProcessError::Cancelled)
    ));
}

#[cfg(unix)]
#[test]
fn process_runner_kills_descendants_on_timeout() {
    let runner = SystemProcessRunner;
    let started = std::time::Instant::now();
    let result = runner.run(
        std::ffi::OsStr::new("sh"),
        &[
            std::ffi::OsString::from("-c"),
            std::ffi::OsString::from("sleep 5 & wait"),
        ],
        &[],
        Duration::from_millis(60),
        &CancellationToken::new(),
    );
    assert!(matches!(
        result,
        Err(yeet_cli::adapters::ProcessError::TimedOut(_))
    ));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "descendant survived timeout cleanup for {:?}",
        started.elapsed()
    );
}

#[cfg(unix)]
#[test]
fn process_runner_bounds_verbose_output_while_draining() {
    let runner = SystemProcessRunner;
    let output = runner
        .run(
            std::ffi::OsStr::new("sh"),
            &[
                std::ffi::OsString::from("-c"),
                std::ffi::OsString::from("head -c 1000000 /dev/zero"),
            ],
            &[],
            Duration::from_secs(5),
            &CancellationToken::new(),
        )
        .unwrap();
    assert!(output.stdout.len() <= 128 * 1024 + 1);
}

#[test]
fn codex_adapter_uses_read_only_ephemeral_schema_invocation() {
    let runner = RecordingRunner::default();
    let observed = runner.clone();
    let adapter = CodexAdapter::with_runner(
        Profile {
            name: "default".to_string(),
            harness: "codex".to_string(),
            model: "gpt-5.6-luna".to_string(),
            reasoning_effort: "low".to_string(),
        },
        tempfile::tempdir().unwrap().path(),
        runner,
    );
    let request = GenerationRequest {
        context: RepositoryContext {
            branch: "main".to_string(),
            staged_tree: "tree".to_string(),
            stat: "file | 1 +".to_string(),
            patch: "+data".to_string(),
            stat_truncation: Default::default(),
            patch_truncation: Default::default(),
        },
        style: ResolvedStyle {
            style: CommitStyle::Repository,
            history: Some(HistoryEvidence {
                examples: "chore: initial".to_string(),
                truncated: false,
                fallback: false,
            }),
            instructions: None,
            instructions_truncated: false,
            fallback: None,
        },
    };
    let draft = adapter.generate(&request).unwrap();
    assert_eq!(draft.subject, "feat: generated safely");
    let args = observed.args.lock().unwrap();
    assert!(args.contains(&"--ephemeral".to_string()));
    assert!(args.contains(&"approval_policy=\"never\"".to_string()));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--sandbox" && pair[1] == "read-only")
    );
    assert!(args.contains(&"--output-schema".to_string()));
    assert!(args.contains(&"--output-last-message".to_string()));
    assert!(
        String::from_utf8(observed.prompt.lock().unwrap().clone())
            .unwrap()
            .contains("exactly one JSON object")
    );
}
