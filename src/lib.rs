// SPDX-License-Identifier: GPL-3.0-or-later

//! Yeet's reusable application boundary.

use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, Once, OnceLock};

pub mod adapters;
pub mod application;
pub mod cli;
pub mod config;
pub mod domain;
pub mod generation;
pub mod ui;

use adapters::{CancellationToken, CodexAdapter, ProcessRunner, SystemGit, SystemProcessRunner};
use application::{WorkflowError, WorkflowOptions, WorkflowOutcome, run_workflow};
use cli::{Cli, Command, ConfigAction, StyleArg};
use config::ResolvedConfig;
use ui::{ConsoleUi, stdin_is_interactive};

#[derive(Debug)]
pub enum YeetError {
    Config(config::ConfigError),
    Workflow(WorkflowError),
    Process(adapters::ProcessError),
    CurrentDirectory(String),
}

impl std::fmt::Display for YeetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => error.fmt(f),
            Self::Workflow(error) => error.fmt(f),
            Self::Process(error) => error.fmt(f),
            Self::CurrentDirectory(message) => {
                write!(f, "cannot determine current directory: {message}")
            }
        }
    }
}
impl std::error::Error for YeetError {}
impl From<config::ConfigError> for YeetError {
    fn from(error: config::ConfigError) -> Self {
        Self::Config(error)
    }
}
impl From<WorkflowError> for YeetError {
    fn from(error: WorkflowError) -> Self {
        Self::Workflow(error)
    }
}
impl From<adapters::ProcessError> for YeetError {
    fn from(error: adapters::ProcessError) -> Self {
        Self::Process(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunOutcome {
    Workflow(WorkflowOutcome),
    ConfigChanged(Vec<String>),
    ConfigShown(String),
}

/// Parse and run the CLI. This function performs no paid model call unless the
/// caller invokes generated mode and has an authenticated Codex executable.
pub fn run(cli: Cli) -> Result<RunOutcome, YeetError> {
    let cwd = env::current_dir().map_err(|error| YeetError::CurrentDirectory(error.to_string()))?;
    if let Some(Command::Config(config_command)) = &cli.command {
        let config_arg = cli.config.as_deref();
        return match &config_command.action {
            ConfigAction::Init => Ok(RunOutcome::ConfigChanged(config::init(config_arg, &cwd)?)),
            ConfigAction::Show => {
                let resolved = resolve_config(&cli, config_arg, &cwd)?;
                Ok(RunOutcome::ConfigShown(config::show(&resolved)))
            }
        };
    }

    let resolved = resolve_config(&cli, cli.config.as_deref(), &cwd)?;
    let manual_message = if cli.message.is_empty() {
        None
    } else {
        Some(cli.message.join(" "))
    };
    let generated = manual_message.is_none();
    let mut ui = ConsoleUi::default();
    let git = SystemGit::new(&cwd);
    let instructions_file = cli
        .instructions_file
        .as_deref()
        .map(|path| config::resolve_cli_instructions_path(path, &cwd));
    if generated {
        if !cli.yes && !stdin_is_interactive() {
            return Err(YeetError::Workflow(WorkflowError::NonInteractive));
        }
        SystemProcessRunner.check_available(std::ffi::OsStr::new("codex"))?;
        let adapter = CodexAdapter::new(resolved.profile.clone(), cwd.clone());
        let cancellation = adapter.cancellation_token();
        install_cancellation_handler(cancellation.clone());
        ui.set_cancellation(cancellation);
        let options = workflow_options(&cli, &resolved, manual_message, instructions_file);
        let outcome = run_workflow(&git, Some(&adapter), &options, &mut ui)?;
        Ok(RunOutcome::Workflow(outcome))
    } else {
        if !cli.yes && stdin_is_interactive() {
            let cancellation = CancellationToken::new();
            install_cancellation_handler(cancellation.clone());
            ui.set_cancellation(cancellation);
        }
        let options = workflow_options(&cli, &resolved, manual_message, instructions_file);
        let outcome =
            run_workflow::<SystemGit, CodexAdapter, ConsoleUi>(&git, None, &options, &mut ui)?;
        Ok(RunOutcome::Workflow(outcome))
    }
}

fn resolve_config(
    cli: &Cli,
    config_arg: Option<&std::path::Path>,
    cwd: &std::path::Path,
) -> Result<ResolvedConfig, YeetError> {
    let style = cli.style.map(style_string);
    Ok(config::resolve(
        config_arg,
        cli.profile.as_deref(),
        cli.model.as_deref(),
        style,
        cwd,
    )?)
}

fn style_string(style: StyleArg) -> &'static str {
    match style {
        StyleArg::Repository => "repository",
        StyleArg::Conventional => "conventional",
        StyleArg::Custom => "custom",
    }
}

fn workflow_options(
    cli: &Cli,
    resolved: &ResolvedConfig,
    manual_message: Option<String>,
    instructions_file: Option<PathBuf>,
) -> WorkflowOptions {
    WorkflowOptions {
        manual_message,
        style: resolved.style,
        global_instructions_file: resolved.instructions_file.clone(),
        cli_instructions_file: instructions_file,
        one_off_instructions: cli.instructions.clone(),
        limits: resolved.limits.clone(),
        push: resolved.push && !cli.no_push,
        yes: cli.yes,
        interactive: stdin_is_interactive(),
    }
}

static SIGNAL_HANDLER: Once = Once::new();
static ACTIVE_CANCELLATION: OnceLock<Mutex<Option<adapters::CancellationToken>>> = OnceLock::new();

fn install_cancellation_handler(token: adapters::CancellationToken) {
    let active = ACTIVE_CANCELLATION.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = active.lock() {
        *guard = Some(token);
    }
    SIGNAL_HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            if let Some(active) = ACTIVE_CANCELLATION.get()
                && let Ok(guard) = active.lock()
                && let Some(token) = guard.as_ref()
            {
                token.cancel();
            }
        });
    });
}
