// SPDX-License-Identifier: GPL-3.0-or-later

//! Harness-independent message generation interfaces.

pub mod prompt;
pub mod style;

use std::fmt;

use crate::domain::{CommitDraft, CommitPolicyError, RepositoryContext};

pub use prompt::{GenerationRequest, build_prompt};
pub use style::{HistoryEvidence, ResolvedStyle, StyleError, resolve_generated_style};

/// A generator receives staged context and returns only a commit draft. It has
/// no access to Git workflow operations.
pub trait MessageGenerator {
    fn generate(&self, request: &GenerationRequest) -> Result<CommitDraft, GenerationError>;
}

#[derive(Debug)]
pub enum GenerationError {
    Process(String),
    Cancelled,
    InvalidResponse(CommitPolicyError),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process(message) => f.write_str(message),
            Self::Cancelled => f.write_str("message generation cancelled"),
            Self::InvalidResponse(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for GenerationError {}

/// Keep the public request construction obvious to adapters and tests.
pub fn request(context: RepositoryContext, style: ResolvedStyle) -> GenerationRequest {
    GenerationRequest { context, style }
}
