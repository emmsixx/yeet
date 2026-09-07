// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::CommitStyle;

const MAX_INSTRUCTION_CODE_POINTS: usize = 12_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEvidence {
    pub examples: String,
    pub truncated: bool,
    pub fallback: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedStyle {
    pub style: CommitStyle,
    pub history: Option<HistoryEvidence>,
    pub instructions: Option<String>,
    pub instructions_truncated: bool,
    pub fallback: Option<String>,
}

impl ResolvedStyle {
    pub fn description(&self) -> &'static str {
        match self.style {
            CommitStyle::Repository => "repository conventions",
            CommitStyle::Conventional => "Conventional Commits",
            CommitStyle::Custom => "custom instructions",
        }
    }
}

#[derive(Debug)]
pub enum StyleError {
    ReadInstructions { path: PathBuf, source: io::Error },
    InvalidInstructionsUtf8 { path: PathBuf },
    MissingInstructions { path: PathBuf },
    CustomInstructionsRequired,
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadInstructions { path, source } => write!(
                f,
                "cannot read instructions file {}: {source}",
                path.display()
            ),
            Self::InvalidInstructionsUtf8 { path } => {
                write!(f, "instructions file {} is not valid UTF-8", path.display())
            }
            Self::MissingInstructions { path } => write!(
                f,
                "configured instructions file {} does not exist",
                path.display()
            ),
            Self::CustomInstructionsRequired => f.write_str(
                "custom style requires non-whitespace instructions from a file or --instructions",
            ),
        }
    }
}

impl std::error::Error for StyleError {}

/// Resolve instructions only for generated mode. Manual messages intentionally
/// bypass this function, history collection, and all harness work.
pub fn resolve_generated_style(
    style: CommitStyle,
    global_instructions_file: Option<&Path>,
    cli_instructions_file: Option<&Path>,
    one_off: Option<&str>,
    history: Option<HistoryEvidence>,
) -> Result<ResolvedStyle, StyleError> {
    let selected_file = cli_instructions_file.or(global_instructions_file);
    let mut instructions = selected_file.map(read_instructions).transpose()?.flatten();
    if let Some(extra) = one_off.filter(|value| !value.trim().is_empty()) {
        let extra = bounded_instruction(extra);
        instructions = Some(match instructions {
            Some(existing) if !existing.trim().is_empty() => {
                format!("{existing}\n\nOne-off guidance:\n{extra}")
            }
            _ => extra,
        });
    }

    if style == CommitStyle::Custom
        && instructions
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        return Err(StyleError::CustomInstructionsRequired);
    }

    let fallback = if style == CommitStyle::Repository
        && history.as_ref().map(|value| value.fallback).unwrap_or(true)
    {
        Some("No eligible commit history is available; use a concise imperative plain-text subject and an optional explanatory body.".to_string())
    } else {
        None
    };
    let instructions_truncated = instructions
        .as_deref()
        .map(|value| value.contains("[instructions truncated"))
        .unwrap_or(false);
    Ok(ResolvedStyle {
        style,
        history,
        instructions,
        instructions_truncated,
        fallback,
    })
}

fn read_instructions(path: &Path) -> Result<Option<String>, StyleError> {
    if !path.exists() {
        return Err(StyleError::MissingInstructions {
            path: path.to_path_buf(),
        });
    }
    let bytes = fs::read(path).map_err(|source| StyleError::ReadInstructions {
        path: path.to_path_buf(),
        source,
    })?;
    let text = String::from_utf8(bytes).map_err(|_| StyleError::InvalidInstructionsUtf8 {
        path: path.to_path_buf(),
    })?;
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(bounded_instruction(&text)))
    }
}

fn bounded_instruction(text: &str) -> String {
    let marker = "\n[instructions truncated to 12000 Unicode code points]";
    let text_code_points = text.chars().count();
    if text_code_points <= MAX_INSTRUCTION_CODE_POINTS {
        return text.to_string();
    }
    let marker_code_points = marker.chars().count();
    let retained_limit = MAX_INSTRUCTION_CODE_POINTS.saturating_sub(marker_code_points);
    let retained: String = text.chars().take(retained_limit).collect();
    if marker_code_points <= MAX_INSTRUCTION_CODE_POINTS {
        format!("{retained}{marker}")
    } else {
        marker.chars().take(MAX_INSTRUCTION_CODE_POINTS).collect()
    }
}
