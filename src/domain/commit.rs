// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// The only message shape accepted from a generator or the manual source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitDraft {
    pub subject: String,
    pub body: String,
}

impl CommitDraft {
    pub fn new(subject: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            body: body.into(),
        }
    }

    pub fn full_message(&self) -> String {
        if self.body.trim().is_empty() {
            self.subject.clone()
        } else {
            format!("{}\n\n{}", self.subject, self.body)
        }
    }
}

/// The writing policy selected independently from a harness/model profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitStyle {
    Repository,
    Conventional,
    Custom,
}

impl CommitStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Repository => "repository",
            Self::Conventional => "conventional",
            Self::Custom => "custom",
        }
    }
}

impl std::str::FromStr for CommitStyle {
    type Err = CommitPolicyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "repository" => Ok(Self::Repository),
            "conventional" => Ok(Self::Conventional),
            "custom" => Ok(Self::Custom),
            other => Err(CommitPolicyError::UnknownStyle(other.to_string())),
        }
    }
}

impl fmt::Display for CommitStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CommitPolicyError {
    UnknownStyle(String),
    EmptySubject,
    MultilineSubject,
    NulCharacter,
    ConventionalSyntax,
    ConventionalLength,
    ConventionalTrailingPeriod,
    InvalidJson(String),
    ResponseTooLarge { actual: usize, max: usize },
}

impl fmt::Display for CommitPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStyle(style) => write!(
                f,
                "unknown commit style '{style}'; use repository, conventional, or custom"
            ),
            Self::EmptySubject => f.write_str("commit subject must contain non-whitespace text"),
            Self::MultilineSubject => f.write_str("commit subject must be a single line"),
            Self::NulCharacter => f.write_str("commit message must not contain NUL characters"),
            Self::ConventionalSyntax => f.write_str(
                "Conventional Commit subject must match lowercase type(scope)!: description",
            ),
            Self::ConventionalLength => {
                f.write_str("Conventional Commit subject must be at most 72 Unicode code points")
            }
            Self::ConventionalTrailingPeriod => {
                f.write_str("Conventional Commit subject must not end with a period")
            }
            Self::InvalidJson(message) => {
                write!(f, "generator response is not valid commit JSON: {message}")
            }
            Self::ResponseTooLarge { actual, max } => write!(
                f,
                "generator response is {actual} bytes; the maximum is {max}"
            ),
        }
    }
}

impl std::error::Error for CommitPolicyError {}

/// Validate universal message rules and, for Conventional Commits, Yeet's
/// additional mechanical rules.
pub fn validate_draft(draft: &CommitDraft, style: CommitStyle) -> Result<(), CommitPolicyError> {
    if draft.subject.trim().is_empty() {
        return Err(CommitPolicyError::EmptySubject);
    }
    if draft.subject.contains('\n') || draft.subject.contains('\r') {
        return Err(CommitPolicyError::MultilineSubject);
    }
    if draft.subject.contains('\0') || draft.body.contains('\0') {
        return Err(CommitPolicyError::NulCharacter);
    }

    if style == CommitStyle::Conventional {
        if draft.subject.chars().count() > 72 {
            return Err(CommitPolicyError::ConventionalLength);
        }
        if draft.subject.ends_with('.') {
            return Err(CommitPolicyError::ConventionalTrailingPeriod);
        }
        if !is_conventional_subject(&draft.subject) {
            return Err(CommitPolicyError::ConventionalSyntax);
        }
    }
    Ok(())
}

fn is_conventional_subject(subject: &str) -> bool {
    let Some((prefix, description)) = subject.split_once(": ") else {
        return false;
    };
    if description.trim().is_empty() {
        return false;
    }

    let mut rest = prefix;
    if rest.ends_with('!') {
        rest = &rest[..rest.len() - 1];
    }
    if rest.is_empty() {
        return false;
    }

    if let Some(open) = rest.find('(') {
        if !rest.ends_with(')') || open == 0 || open + 2 > rest.len() {
            return false;
        }
        let scope = &rest[open + 1..rest.len() - 1];
        if scope.is_empty()
            || scope
                .chars()
                .any(|c| c == '(' || c == ')' || c.is_whitespace())
        {
            return false;
        }
        rest = &rest[..open];
    }
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

struct GeneratedDraft {
    subject: String,
    body: String,
}

impl<'de> Deserialize<'de> for GeneratedDraft {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GeneratedDraftVisitor;

        impl<'de> Visitor<'de> for GeneratedDraftVisitor {
            type Value = GeneratedDraft;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object with exactly subject and body string fields")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut subject = None;
                let mut body = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "subject" => {
                            if subject.is_some() {
                                return Err(de::Error::duplicate_field("subject"));
                            }
                            subject = Some(map.next_value()?);
                        }
                        "body" => {
                            if body.is_some() {
                                return Err(de::Error::duplicate_field("body"));
                            }
                            body = Some(map.next_value()?);
                        }
                        other => return Err(de::Error::unknown_field(other, &["subject", "body"])),
                    }
                }
                Ok(GeneratedDraft {
                    subject: subject.ok_or_else(|| de::Error::missing_field("subject"))?,
                    body: body.ok_or_else(|| de::Error::missing_field("body"))?,
                })
            }
        }

        deserializer.deserialize_map(GeneratedDraftVisitor)
    }
}

/// Parse and validate the exact two-key JSON response from a generator.
pub fn parse_generated_draft(
    response: &[u8],
    style: CommitStyle,
    max_bytes: usize,
) -> Result<CommitDraft, CommitPolicyError> {
    if response.len() > max_bytes {
        return Err(CommitPolicyError::ResponseTooLarge {
            actual: response.len(),
            max: max_bytes,
        });
    }
    let parsed: GeneratedDraft = serde_json::from_slice(response)
        .map_err(|error| CommitPolicyError::InvalidJson(error.to_string()))?;
    let draft = CommitDraft::new(parsed.subject, parsed.body);
    validate_draft(&draft, style)?;
    Ok(draft)
}
