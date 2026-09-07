// SPDX-License-Identifier: GPL-3.0-or-later

//! Command-line parsing. The CLI deliberately contains no Git or harness
//! behaviour; that wiring lives in [`crate::run`].

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Stage, draft, commit, and push with your preferred writing style.
#[derive(Debug, Parser)]
#[command(
    name = "yeet",
    version,
    about = "Stage changes, draft a commit message, and commit with review",
    after_help = "Generated mode sends bounded staged content to the selected harness.\nUse `yeet config init` to create configuration and `yeet --help` for options."
)]
pub struct Cli {
    /// Select a named generation profile.
    #[arg(long, global = true, env = "YEET_PROFILE")]
    pub profile: Option<String>,

    /// Override the selected profile's model for this invocation.
    #[arg(long, global = true, env = "YEET_MODEL")]
    pub model: Option<String>,

    /// Select the commit writing style.
    #[arg(long, global = true, value_enum, env = "YEET_STYLE")]
    pub style: Option<StyleArg>,

    /// Read configuration from this file.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Replace the configured Markdown instructions file for this invocation.
    #[arg(long, global = true)]
    pub instructions_file: Option<PathBuf>,

    /// Add one-off writing guidance for generated messages.
    #[arg(long, global = true)]
    pub instructions: Option<String>,

    /// Skip the confirmation prompt.
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    /// Commit locally without running `git push`.
    #[arg(long, global = true)]
    pub no_push: bool,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Manual commit message words. With no words Yeet generates a draft.
    #[arg(value_name = "MESSAGE")]
    pub message: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage user configuration and Markdown writing instructions.
    Config(ConfigCommand),
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Create starter config.toml and instructions.md without overwriting files.
    Init,
    /// Print resolved configuration and source paths.
    Show,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum StyleArg {
    Repository,
    Conventional,
    Custom,
}
