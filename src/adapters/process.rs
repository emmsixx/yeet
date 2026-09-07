// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CAPTURED_STDOUT_BYTES: usize = 128 * 1024 + 1;
const MAX_CAPTURED_STDERR_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    pub fn shared(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

#[derive(Debug)]
pub enum ProcessError {
    Spawn {
        executable: String,
        source: io::Error,
    },
    Io(io::Error),
    TimedOut(Duration),
    Cancelled,
    FailedAvailability {
        executable: String,
        detail: String,
    },
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn { executable, source } => write!(f, "cannot start {executable}: {source}"),
            Self::Io(source) => write!(f, "process I/O failed: {source}"),
            Self::TimedOut(timeout) => write!(f, "process timed out after {timeout:?}"),
            Self::Cancelled => f.write_str("process cancelled"),
            Self::FailedAvailability { executable, detail } => {
                write!(f, "cannot use {executable}: {detail}")
            }
        }
    }
}

impl std::error::Error for ProcessError {}

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// External command boundary. Arguments are passed as an array and never
/// through a shell command string.
pub trait ProcessRunner: Send + Sync {
    fn run(
        &self,
        executable: &OsStr,
        args: &[OsString],
        stdin: &[u8],
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<ProcessOutput, ProcessError>;

    fn check_available(&self, executable: &OsStr) -> Result<(), ProcessError>;
}

#[derive(Clone, Debug, Default)]
pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(
        &self,
        executable: &OsStr,
        args: &[OsString],
        stdin: &[u8],
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<ProcessOutput, ProcessError> {
        if cancellation.is_cancelled() {
            return Err(ProcessError::Cancelled);
        }
        let (mut child, grouped) =
            spawn_command(executable, args).map_err(|source| ProcessError::Spawn {
                executable: executable.to_string_lossy().into_owned(),
                source,
            })?;

        let mut child_stdin = child.stdin.take();
        let input = stdin.to_vec();
        let writer = thread::spawn(move || {
            if let Some(mut pipe) = child_stdin.take() {
                let result = pipe.write_all(&input);
                drop(pipe);
                result
            } else {
                Ok(())
            }
        });

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessError::Io(io::Error::other("child stdout was not piped")))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProcessError::Io(io::Error::other("child stderr was not piped")))?;
        let stdout_reader = thread::spawn(move || read_capped(stdout, MAX_CAPTURED_STDOUT_BYTES));
        let stderr_reader = thread::spawn(move || read_capped(stderr, MAX_CAPTURED_STDERR_BYTES));

        let started = Instant::now();
        let mut forced_error = None;
        let status = loop {
            if cancellation.is_cancelled() {
                terminate_process(&mut child, grouped);
                forced_error = Some(ProcessError::Cancelled);
                break child.wait().map_err(ProcessError::Io)?;
            }
            if started.elapsed() >= timeout {
                terminate_process(&mut child, grouped);
                forced_error = Some(ProcessError::TimedOut(timeout));
                break child.wait().map_err(ProcessError::Io)?;
            }
            if let Some(status) = child.try_wait().map_err(ProcessError::Io)? {
                break status;
            }
            thread::sleep(Duration::from_millis(20));
        };

        let writer_result = writer
            .join()
            .map_err(|_| ProcessError::Io(io::Error::other("stdin writer thread panicked")))?;
        let stdout = stdout_reader
            .join()
            .map_err(|_| ProcessError::Io(io::Error::other("stdout reader thread panicked")))?
            .map_err(ProcessError::Io)?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| ProcessError::Io(io::Error::other("stderr reader thread panicked")))?
            .map_err(ProcessError::Io)?;
        if let Some(error) = forced_error {
            return Err(error);
        }
        // A command that exits before consuming all input can close stdin. A
        // broken pipe then accompanies its useful exit status and is harmless.
        if let Err(error) = writer_result
            && !matches!(error.kind(), io::ErrorKind::BrokenPipe)
            && status.success()
        {
            return Err(ProcessError::Io(error));
        }
        Ok(ProcessOutput {
            status,
            stdout,
            stderr,
        })
    }

    fn check_available(&self, executable: &OsStr) -> Result<(), ProcessError> {
        let output = Command::new(executable)
            .arg("--version")
            .output()
            .map_err(|source| ProcessError::Spawn {
                executable: executable.to_string_lossy().into_owned(),
                source,
            })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(ProcessError::FailedAvailability {
                executable: executable.to_string_lossy().into_owned(),
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            })
        }
    }
}

/// Drain a pipe completely while retaining only a bounded prefix. Continuing
/// to read after the retained prefix prevents a verbose child from blocking on
/// a full pipe; the supervisor can still enforce its timeout and cancellation.
fn read_capped(mut reader: impl Read, keep_limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
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
        }
    }
    Ok(bytes)
}

fn spawn_command(executable: &OsStr, args: &[OsString]) -> io::Result<(std::process::Child, bool)> {
    #[cfg(unix)]
    {
        // A fresh process group lets cleanup terminate descendants that
        // inherited output pipes. `process_group(0)` is the stdlib API for
        // this and avoids depending on an external `setsid` utility.
        use std::os::unix::process::CommandExt;
        let mut grouped = Command::new(executable);
        grouped
            .process_group(0)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        grouped.spawn().map(|child| (child, true))
    }
    #[cfg(not(unix))]
    {
        let mut direct = Command::new(executable);
        direct
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        direct.spawn().map(|child| (child, false))
    }
}

fn terminate_process(child: &mut std::process::Child, grouped: bool) {
    #[cfg(unix)]
    if grouped {
        let pid = child.id().to_string();
        let group = format!("-{pid}");
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", group.as_str()])
            .status();
    }
    let _ = child.kill();
}
