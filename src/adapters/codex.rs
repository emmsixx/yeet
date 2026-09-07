// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::Profile;
use crate::domain::{CommitPolicyError, parse_generated_draft};
use crate::generation::{GenerationError, GenerationRequest, MessageGenerator};

use super::process::{CancellationToken, ProcessRunner, SystemProcessRunner};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RESPONSE_BYTES: usize = 128 * 1024;

/// Codex's noninteractive adapter. It asks the authenticated local CLI for a
/// final structured message while keeping the model sandbox read-only and the
/// session ephemeral.
pub struct CodexAdapter<R: ProcessRunner = SystemProcessRunner> {
    profile: Profile,
    cwd: PathBuf,
    runner: R,
    timeout: Duration,
    cancellation: CancellationToken,
}

impl CodexAdapter<SystemProcessRunner> {
    pub fn new(profile: Profile, cwd: impl Into<PathBuf>) -> Self {
        Self {
            profile,
            cwd: cwd.into(),
            runner: SystemProcessRunner,
            timeout: DEFAULT_TIMEOUT,
            cancellation: CancellationToken::new(),
        }
    }
}

impl<R: ProcessRunner> CodexAdapter<R> {
    pub fn with_runner(profile: Profile, cwd: impl Into<PathBuf>, runner: R) -> Self {
        Self {
            profile,
            cwd: cwd.into(),
            runner,
            timeout: DEFAULT_TIMEOUT,
            cancellation: CancellationToken::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn model(&self) -> &str {
        &self.profile.model
    }

    fn schema_file(directory: &Path) -> Result<PathBuf, GenerationError> {
        let path = directory.join("schema.json");
        let schema = r#"{
  "type": "object",
  "additionalProperties": false,
  "required": ["subject", "body"],
  "properties": {
    "subject": { "type": "string" },
    "body": { "type": "string" }
  }
}
"#;
        fs::write(&path, schema).map_err(|error| {
            GenerationError::Process(format!(
                "cannot create temporary Codex output schema: {error}"
            ))
        })?;
        Ok(path)
    }
}

impl<R: ProcessRunner> MessageGenerator for CodexAdapter<R> {
    fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<crate::domain::CommitDraft, GenerationError> {
        let temporary = tempfile::tempdir().map_err(|error| {
            GenerationError::Process(format!("cannot create temporary Codex directory: {error}"))
        })?;
        let schema = Self::schema_file(temporary.path())?;
        let output_path = temporary.path().join("last-message.txt");
        let args = codex_args_for(&self.profile, &self.cwd, &schema, &output_path);
        let prompt = crate::generation::build_prompt(request);
        let process = self
            .runner
            .run(
                OsStr::new("codex"),
                &args,
                prompt.as_bytes(),
                self.timeout,
                &self.cancellation,
            )
            .map_err(|error| match error {
                super::process::ProcessError::Cancelled => GenerationError::Cancelled,
                other => GenerationError::Process(format!("Codex generation failed: {other}")),
            })?;
        if self.cancellation.is_cancelled() {
            return Err(GenerationError::Cancelled);
        }
        if !process.status.success() {
            let detail = String::from_utf8_lossy(&process.stderr).trim().to_string();
            return Err(GenerationError::Process(if detail.is_empty() {
                format!("Codex exited with status {}", process.status)
            } else {
                format!("Codex exited with status {}: {detail}", process.status)
            }));
        }
        let response = if let Ok(metadata) = fs::metadata(&output_path) {
            if metadata.len() > MAX_RESPONSE_BYTES as u64 {
                return Err(GenerationError::InvalidResponse(
                    CommitPolicyError::ResponseTooLarge {
                        actual: metadata.len() as usize,
                        max: MAX_RESPONSE_BYTES,
                    },
                ));
            }
            fs::read(&output_path).map_err(|error| {
                GenerationError::Process(format!("cannot read Codex final message: {error}"))
            })?
        } else if !process.stdout.is_empty() {
            // The supported CLI writes the final response with -o. Keeping a
            // bounded stdout fallback makes the adapter useful with compatible
            // wrappers and simple test harnesses that emit only the final JSON.
            process.stdout
        } else {
            return Err(GenerationError::Process(
                "Codex did not produce a final message".to_string(),
            ));
        };
        parse_generated_draft(&response, request.style.style, MAX_RESPONSE_BYTES)
            .map_err(GenerationError::InvalidResponse)
    }
}

/// Keep this helper available to tests that need to assert the adapter's
/// command construction without invoking a model.
pub fn codex_args_for(
    profile: &Profile,
    cwd: &Path,
    schema: &Path,
    output: &Path,
) -> Vec<OsString> {
    vec![
        OsString::from("exec"),
        OsString::from("--ephemeral"),
        OsString::from("--sandbox"),
        OsString::from("read-only"),
        OsString::from("--ignore-rules"),
        OsString::from("-c"),
        OsString::from("approval_policy=\"never\""),
        OsString::from("--model"),
        OsString::from(&profile.model),
        OsString::from("-c"),
        OsString::from(format!(
            "model_reasoning_effort=\"{}\"",
            profile.reasoning_effort
        )),
        OsString::from("--cd"),
        cwd.as_os_str().into(),
        OsString::from("--output-last-message"),
        output.as_os_str().into(),
        OsString::from("--output-schema"),
        schema.as_os_str().into(),
        OsString::from("--color"),
        OsString::from("never"),
        OsString::from("-"),
    ]
}
