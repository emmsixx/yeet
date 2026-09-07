// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use crate::config::ContextLimits;
use crate::domain::{RepositoryContext, Truncation};
use crate::generation::HistoryEvidence;

const MAX_CAPTURED_STDERR_BYTES: usize = 64 * 1024;
const HISTORY_MAX_CHARS: usize = 12_000;

pub trait GitGateway {
    fn verify_worktree(&self) -> Result<(), GitError>;
    fn stage_all(&self) -> Result<(), GitError>;
    fn has_staged_changes(&self) -> Result<bool, GitError>;
    fn staged_tree_id(&self) -> Result<String, GitError>;
    fn repository_context(&self, limits: &ContextLimits) -> Result<RepositoryContext, GitError>;
    fn history_evidence(&self) -> Result<HistoryEvidence, GitError>;
    fn commit(&self, message: &str) -> Result<String, GitError>;
    fn push(&self) -> Result<(), GitError>;
}

#[derive(Debug)]
pub enum GitError {
    Command {
        operation: String,
        status: Option<i32>,
        stderr: String,
    },
    Io {
        operation: String,
        message: String,
    },
    NotWorktree,
    NoStagedChanges,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command {
                operation,
                status,
                stderr,
            } => {
                if let Some(status) = status {
                    write!(f, "git {operation} failed with exit status {status}")?;
                } else {
                    write!(f, "git {operation} failed")?;
                }
                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
            Self::Io { operation, message } => {
                write!(f, "could not run git {operation}: {message}")
            }
            Self::NotWorktree => f.write_str("the current directory is not inside a Git worktree"),
            Self::NoStagedChanges => f.write_str("stage-all produced no staged changes"),
        }
    }
}

impl std::error::Error for GitError {}

#[derive(Clone, Debug)]
pub struct SystemGit {
    cwd: PathBuf,
}

impl SystemGit {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    fn command(&self, operation: &str, args: &[&str]) -> Result<GitOutput, GitError> {
        let output = Command::new("git")
            .current_dir(&self.cwd)
            .args(args)
            .output()
            .map_err(|error| GitError::Io {
                operation: operation.to_string(),
                message: error.to_string(),
            })?;
        Ok(GitOutput {
            status: output.status.code(),
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn require_success(&self, operation: &str, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let output = self.command(operation, args)?;
        if output.success {
            Ok(output.stdout)
        } else {
            Err(GitError::Command {
                operation: operation.to_string(),
                status: output.status,
                stderr: lossy_trim(&output.stderr),
            })
        }
    }

    fn command_with_limits(
        &self,
        operation: &str,
        args: &[&str],
        stdout_limit: usize,
        stderr_limit: usize,
    ) -> Result<LimitedGitOutput, GitError> {
        let mut child = Command::new("git")
            .current_dir(&self.cwd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| GitError::Io {
                operation: operation.to_string(),
                message: error.to_string(),
            })?;
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError::Io {
                    operation: operation.to_string(),
                    message: "git stdout was not piped".to_string(),
                });
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError::Io {
                    operation: operation.to_string(),
                    message: "git stderr was not piped".to_string(),
                });
            }
        };
        let stdout_reader = thread::spawn(move || read_capped(stdout, stdout_limit));
        let stderr_reader = thread::spawn(move || read_capped(stderr, stderr_limit));
        let status = child.wait().map_err(|error| GitError::Io {
            operation: operation.to_string(),
            message: error.to_string(),
        })?;
        let stdout = stdout_reader
            .join()
            .map_err(|_| GitError::Io {
                operation: operation.to_string(),
                message: "git stdout reader thread panicked".to_string(),
            })?
            .map_err(|error| GitError::Io {
                operation: operation.to_string(),
                message: error.to_string(),
            })?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| GitError::Io {
                operation: operation.to_string(),
                message: "git stderr reader thread panicked".to_string(),
            })?
            .map_err(|error| GitError::Io {
                operation: operation.to_string(),
                message: error.to_string(),
            })?;
        Ok(LimitedGitOutput {
            status: status.code(),
            success: status.success(),
            stdout,
            stderr,
        })
    }

    fn require_success_with_limits(
        &self,
        operation: &str,
        args: &[&str],
        stdout_limit: usize,
    ) -> Result<CapturedBytes, GitError> {
        let output =
            self.command_with_limits(operation, args, stdout_limit, MAX_CAPTURED_STDERR_BYTES)?;
        if output.success {
            Ok(output.stdout)
        } else {
            Err(GitError::Command {
                operation: operation.to_string(),
                status: output.status,
                stderr: lossy_trim(&output.stderr.bytes),
            })
        }
    }
}

struct GitOutput {
    status: Option<i32>,
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct LimitedGitOutput {
    status: Option<i32>,
    success: bool,
    stdout: CapturedBytes,
    stderr: CapturedBytes,
}

struct CapturedBytes {
    bytes: Vec<u8>,
    truncated: bool,
}

impl GitGateway for SystemGit {
    fn verify_worktree(&self) -> Result<(), GitError> {
        let output = self.command("rev-parse", &["rev-parse", "--is-inside-work-tree"])?;
        if output.success && String::from_utf8_lossy(&output.stdout).trim() == "true" {
            Ok(())
        } else if output.success {
            Err(GitError::NotWorktree)
        } else {
            let stderr = lossy_trim(&output.stderr);
            if stderr.to_ascii_lowercase().contains("not a git repository") {
                return Err(GitError::NotWorktree);
            }
            Err(GitError::Command {
                operation: "rev-parse --is-inside-work-tree".to_string(),
                status: output.status,
                stderr,
            })
        }
    }

    fn stage_all(&self) -> Result<(), GitError> {
        self.require_success("add -A", &["add", "-A", "--"])
            .map(|_| ())
    }

    fn has_staged_changes(&self) -> Result<bool, GitError> {
        let output = self.command(
            "diff --cached --quiet",
            &["diff", "--cached", "--quiet", "--exit-code", "--"],
        )?;
        match output.status {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(GitError::Command {
                operation: "diff --cached --quiet".to_string(),
                status: output.status,
                stderr: lossy_trim(&output.stderr),
            }),
        }
    }

    fn staged_tree_id(&self) -> Result<String, GitError> {
        let bytes = self.require_success("write-tree", &["write-tree"])?;
        let tree = String::from_utf8_lossy(&bytes).trim().to_string();
        if tree.is_empty() {
            Err(GitError::Command {
                operation: "write-tree".to_string(),
                status: Some(1),
                stderr: "Git returned an empty staged tree ID".to_string(),
            })
        } else {
            Ok(tree)
        }
    }

    fn repository_context(&self, limits: &ContextLimits) -> Result<RepositoryContext, GitError> {
        let branch = self.branch()?;
        let staged_tree = self.staged_tree_id()?;
        let stat = self.require_success_with_limits(
            "diff --cached --stat",
            &[
                "diff",
                "--cached",
                "--stat",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--",
            ],
            capture_limit(limits.stat_max_chars),
        )?;
        let patch = self.require_success_with_limits(
            "diff --cached",
            &[
                "diff",
                "--cached",
                "--binary",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--",
            ],
            capture_limit(limits.patch_max_chars),
        )?;
        let (stat, stat_truncation) =
            bounded_lossy_with_capture(&stat.bytes, limits.stat_max_chars, stat.truncated);
        let (patch, patch_truncation) =
            bounded_lossy_with_capture(&patch.bytes, limits.patch_max_chars, patch.truncated);
        Ok(RepositoryContext {
            branch,
            staged_tree,
            stat,
            patch,
            stat_truncation,
            patch_truncation,
        })
    }

    fn history_evidence(&self) -> Result<HistoryEvidence, GitError> {
        let output = self.command_with_limits(
            "log",
            &["log", "--no-merges", "-n", "20", "--format=%B%x00", "HEAD"],
            capture_limit(HISTORY_MAX_CHARS),
            MAX_CAPTURED_STDERR_BYTES,
        )?;
        if !output.success {
            let stderr = lossy_trim(&output.stderr.bytes);
            let lowercase = stderr.to_ascii_lowercase();
            if lowercase.contains("does not have any commits yet")
                || lowercase.contains("ambiguous argument 'head'")
                || (lowercase.contains("your current branch")
                    && lowercase.contains("does not have any commits"))
            {
                return Ok(empty_history());
            }
            return Err(GitError::Command {
                operation: "log --no-merges".to_string(),
                status: output.status,
                stderr,
            });
        }
        let mut examples = String::new();
        let mut retained = 0usize;
        let mut truncated = false;
        for raw in String::from_utf8_lossy(&output.stdout.bytes).split('\0') {
            let message = raw.trim();
            if message.is_empty() {
                continue;
            }
            let separator = if examples.is_empty() { "" } else { "\n---\n" };
            let needed = separator.chars().count() + message.chars().count();
            if retained + needed > HISTORY_MAX_CHARS {
                let available =
                    HISTORY_MAX_CHARS.saturating_sub(retained + separator.chars().count());
                if available > 0 {
                    examples.push_str(separator);
                    examples.extend(message.chars().take(available));
                }
                truncated = true;
                break;
            }
            examples.push_str(separator);
            examples.push_str(message);
            retained += needed;
        }
        if truncated || output.stdout.truncated {
            examples = with_bounded_marker(
                &examples,
                HISTORY_MAX_CHARS,
                "[history examples truncated to 12000 Unicode code points]",
            );
            truncated = true;
        }
        if examples.is_empty() {
            Ok(empty_history())
        } else {
            Ok(HistoryEvidence {
                examples,
                truncated,
                fallback: false,
            })
        }
    }

    fn commit(&self, message: &str) -> Result<String, GitError> {
        let subject = message
            .split_once("\n\n")
            .map(|(subject, _)| subject)
            .unwrap_or(message);
        let body = message
            .split_once("\n\n")
            .map(|(_, body)| body)
            .unwrap_or("");
        let mut args = vec!["commit", "-m", subject];
        if !body.is_empty() {
            args.extend(["-m", body]);
        }
        self.require_success("commit", &args)?;
        let bytes = self.require_success("rev-parse HEAD", &["rev-parse", "HEAD"])?;
        let id = String::from_utf8_lossy(&bytes).trim().to_string();
        if id.is_empty() {
            Err(GitError::Command {
                operation: "rev-parse HEAD".to_string(),
                status: Some(1),
                stderr: "Git returned an empty commit ID".to_string(),
            })
        } else {
            Ok(id)
        }
    }

    fn push(&self) -> Result<(), GitError> {
        self.require_success("push", &["push"]).map(|_| ())
    }
}

impl SystemGit {
    fn branch(&self) -> Result<String, GitError> {
        let symbolic = self.command("symbolic-ref", &["symbolic-ref", "--short", "HEAD"])?;
        if symbolic.success {
            return Ok(String::from_utf8_lossy(&symbolic.stdout).trim().to_string());
        }
        // Detached HEAD is valid; rev-parse reports HEAD in that case.
        if symbolic.status != Some(1) {
            return Err(GitError::Command {
                operation: "symbolic-ref --short HEAD".to_string(),
                status: symbolic.status,
                stderr: lossy_trim(&symbolic.stderr),
            });
        }
        let detached = self.require_success(
            "rev-parse --abbrev-ref HEAD",
            &["rev-parse", "--abbrev-ref", "HEAD"],
        )?;
        Ok(String::from_utf8_lossy(&detached).trim().to_string())
    }
}

fn empty_history() -> HistoryEvidence {
    HistoryEvidence {
        examples: "No eligible non-merge commit history is available.".to_string(),
        truncated: false,
        fallback: true,
    }
}

fn lossy_trim(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn bounded_lossy_with_capture(
    bytes: &[u8],
    max_code_points: usize,
    captured_truncated: bool,
) -> (String, Truncation) {
    let text = String::from_utf8_lossy(bytes);
    let original_code_points = text.chars().count();
    if !captured_truncated && original_code_points <= max_code_points {
        return (
            text.into_owned(),
            Truncation {
                truncated: false,
                original_code_points,
                retained_code_points: original_code_points,
            },
        );
    }
    let marker = format!("\n[context truncated to {max_code_points} Unicode code points]");
    let marker_code_points = marker.chars().count();
    let retained_limit = max_code_points.saturating_sub(marker_code_points);
    let retained: String = text.chars().take(retained_limit).collect();
    let retained_code_points = retained.chars().count();
    let value = if marker_code_points <= max_code_points {
        format!("{retained}{marker}")
    } else {
        text.chars().take(max_code_points).collect()
    };
    (
        value,
        Truncation {
            truncated: true,
            original_code_points: if captured_truncated {
                original_code_points.max(max_code_points.saturating_add(1))
            } else {
                original_code_points
            },
            retained_code_points,
        },
    )
}

fn capture_limit(max_code_points: usize) -> usize {
    max_code_points
        .saturating_add(64)
        .saturating_mul(4)
        .saturating_add(4)
}

/// Drain a Git pipe completely while retaining a bounded prefix. This keeps a
/// large staged patch from becoming an unbounded allocation or deadlocking Git
/// when the retained context budget has already been reached.
fn read_capped(mut reader: impl Read, keep_limit: usize) -> io::Result<CapturedBytes> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if read == 0 {
            break;
        }
        if bytes.len() < keep_limit {
            let retained = (keep_limit - bytes.len()).min(read);
            bytes.extend_from_slice(&buffer[..retained]);
            if retained < read {
                truncated = true;
            }
        } else {
            truncated = true;
        }
    }
    Ok(CapturedBytes { bytes, truncated })
}

fn with_bounded_marker(text: &str, max_code_points: usize, marker: &str) -> String {
    let marker_with_newline = format!("\n{marker}");
    let marker_code_points = marker_with_newline.chars().count();
    if marker_code_points >= max_code_points {
        return marker_with_newline.chars().take(max_code_points).collect();
    }
    let retained_limit = max_code_points - marker_code_points;
    let retained: String = text.chars().take(retained_limit).collect();
    format!("{retained}{marker_with_newline}")
}
