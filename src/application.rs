// SPDX-License-Identifier: GPL-3.0-or-later

//! The Git workflow state machine. All external operations are expressed as
//! traits so tests can use temporary repositories and fakes.

use std::path::PathBuf;

use crate::adapters::{GitError, GitGateway};
use crate::config::ContextLimits;
use crate::domain::{CommitDraft, CommitPolicyError, CommitStyle, validate_draft};
use crate::generation::{GenerationError, MessageGenerator, StyleError, resolve_generated_style};

pub struct WorkflowOptions {
    pub manual_message: Option<String>,
    pub style: CommitStyle,
    pub global_instructions_file: Option<PathBuf>,
    pub cli_instructions_file: Option<PathBuf>,
    pub one_off_instructions: Option<String>,
    pub limits: ContextLimits,
    pub push: bool,
    pub yes: bool,
    pub interactive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowOutcome {
    Committed { commit_id: String, pushed: bool },
    Cancelled,
}

#[derive(Debug)]
pub enum WorkflowError {
    Git(GitError),
    Style(StyleError),
    Generation(GenerationError),
    Policy(CommitPolicyError),
    NonInteractive,
    StagedTreeChanged { before: String, after: String },
    PushFailed { commit_id: String, source: GitError },
    MissingGenerator,
    Ui(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git(error) => error.fmt(f),
            Self::Style(error) => error.fmt(f),
            Self::Generation(error) => error.fmt(f),
            Self::Policy(error) => error.fmt(f),
            Self::NonInteractive => f.write_str(
                "stdin is not interactive; rerun with --yes to approve the commit before staging",
            ),
            Self::StagedTreeChanged { before, after } => write!(
                f,
                "staged tree changed during Yeet workflow ({before} became {after}); no commit was made"
            ),
            Self::PushFailed { commit_id, source } => write!(
                f,
                "commit {commit_id} succeeded locally, but push failed: {source}"
            ),
            Self::MissingGenerator => {
                f.write_str("generated mode requires the selected harness adapter")
            }
            Self::Ui(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for WorkflowError {}

impl From<GitError> for WorkflowError {
    fn from(error: GitError) -> Self {
        Self::Git(error)
    }
}
impl From<StyleError> for WorkflowError {
    fn from(error: StyleError) -> Self {
        Self::Style(error)
    }
}
impl From<GenerationError> for WorkflowError {
    fn from(error: GenerationError) -> Self {
        Self::Generation(error)
    }
}
impl From<CommitPolicyError> for WorkflowError {
    fn from(error: CommitPolicyError) -> Self {
        Self::Policy(error)
    }
}

/// UI operations stay outside the workflow logic, allowing tests to approve or
/// cancel without a terminal.
pub trait WorkflowUi {
    fn progress(&mut self, message: &str);
    fn preview(
        &mut self,
        style: CommitStyle,
        fallback: Option<&str>,
        stat: &str,
        draft: &CommitDraft,
    );
    fn confirm(&mut self) -> Result<bool, String>;
    fn committed(&mut self, commit_id: &str, pushed: bool);
    fn cancelled(&mut self);
}

pub fn run_workflow<
    G: GitGateway + ?Sized,
    M: MessageGenerator + ?Sized,
    U: WorkflowUi + ?Sized,
>(
    git: &G,
    generator: Option<&M>,
    options: &WorkflowOptions,
    ui: &mut U,
) -> Result<WorkflowOutcome, WorkflowError> {
    let generated = options.manual_message.is_none();
    let manual = options
        .manual_message
        .as_deref()
        .map(|message| CommitDraft::new(message, ""));
    if let Some(draft) = &manual {
        validate_draft(draft, options.style)?;
    }
    if !options.yes && !options.interactive {
        return Err(WorkflowError::NonInteractive);
    }
    if generated && generator.is_none() {
        return Err(WorkflowError::MissingGenerator);
    }

    // Validate the worktree before collecting any repository context or
    // history. This keeps failures outside Git repositories actionable.
    git.verify_worktree()?;

    // Repository examples are collected before staging so stage-all itself
    // cannot change the style evidence. Other styles do not inspect history.
    let history = if generated && options.style == CommitStyle::Repository {
        Some(git.history_evidence()?)
    } else {
        None
    };
    let resolved_style = if generated {
        Some(resolve_generated_style(
            options.style,
            options.global_instructions_file.as_deref(),
            options.cli_instructions_file.as_deref(),
            options.one_off_instructions.as_deref(),
            history,
        )?)
    } else {
        None
    };

    ui.progress("staging all changes");
    git.stage_all()?;
    if !git.has_staged_changes()? {
        return Err(GitError::NoStagedChanges.into());
    }
    let tree_before = git.staged_tree_id()?;
    let context = git.repository_context(&options.limits)?;
    if context.staged_tree != tree_before {
        return Err(WorkflowError::StagedTreeChanged {
            before: tree_before,
            after: context.staged_tree.clone(),
        });
    }

    let draft = if let Some(draft) = manual {
        draft
    } else {
        ui.progress("drafting with the selected harness");
        generator
            .expect("checked above")
            .generate(&crate::generation::request(
                context.clone(),
                resolved_style.clone().expect("generated style"),
            ))?
    };
    validate_draft(&draft, options.style)?;
    let tree_after_generation = git.staged_tree_id()?;
    if tree_after_generation != tree_before {
        return Err(WorkflowError::StagedTreeChanged {
            before: tree_before,
            after: tree_after_generation,
        });
    }

    ui.preview(
        options.style,
        resolved_style
            .as_ref()
            .and_then(|style| style.fallback.as_deref()),
        &context.stat,
        &draft,
    );
    if !options.yes && !ui.confirm().map_err(WorkflowError::Ui)? {
        ui.cancelled();
        return Ok(WorkflowOutcome::Cancelled);
    }
    let tree_before_commit = git.staged_tree_id()?;
    if tree_before_commit != tree_before {
        return Err(WorkflowError::StagedTreeChanged {
            before: tree_before,
            after: tree_before_commit,
        });
    }

    let commit_id = git.commit(&draft.full_message())?;
    let pushed = if options.push {
        ui.progress("pushing commit");
        match git.push() {
            Ok(()) => true,
            Err(source) => return Err(WorkflowError::PushFailed { commit_id, source }),
        }
    } else {
        false
    };
    ui.committed(&commit_id, pushed);
    Ok(WorkflowOutcome::Committed { commit_id, pushed })
}
