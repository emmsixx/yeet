// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{self, IsTerminal, Write};
use std::sync::mpsc;
use std::time::Duration;

use crate::adapters::CancellationToken;
use crate::application::WorkflowUi;
use crate::domain::{CommitDraft, CommitStyle};

#[derive(Default)]
pub struct ConsoleUi {
    cancellation: CancellationToken,
}

impl ConsoleUi {
    pub fn set_cancellation(&mut self, cancellation: CancellationToken) {
        self.cancellation = cancellation;
    }
}

impl WorkflowUi for ConsoleUi {
    fn progress(&mut self, message: &str) {
        eprintln!("yeet: {message}");
    }

    fn preview(
        &mut self,
        style: CommitStyle,
        fallback: Option<&str>,
        stat: &str,
        draft: &CommitDraft,
    ) {
        println!("Staged changes ({style} style):");
        if stat.trim().is_empty() {
            println!("  (stat unavailable)");
        } else {
            println!("{stat}");
        }
        if let Some(fallback) = fallback {
            println!("Style fallback: {fallback}");
        }
        println!(
            "\nProposed commit message:\n---\n{}\n---",
            draft.full_message()
        );
        let _ = io::stdout().flush();
    }

    fn confirm(&mut self) -> Result<bool, String> {
        eprint!("Press Enter to commit, or Ctrl+C/EOF to cancel: ");
        io::stderr().flush().map_err(|error| error.to_string())?;
        if self.cancellation.is_cancelled() {
            return Ok(false);
        }

        // A Ctrl+C handler runs on another thread and may not interrupt a
        // blocking terminal read on every platform. Keep the read isolated so
        // the workflow can observe the token promptly and leave the staged
        // index untouched on cancellation.
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut answer = String::new();
            let result = match io::stdin().read_line(&mut answer) {
                Ok(read) => Ok(read > 0 && answer.trim_end_matches(['\r', '\n']).is_empty()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(false),
                Err(error) => Err(error.to_string()),
            };
            let _ = sender.send(result);
        });
        loop {
            if self.cancellation.is_cancelled() {
                return Ok(false);
            }
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(result) => {
                    return if self.cancellation.is_cancelled() {
                        Ok(false)
                    } else {
                        result
                    };
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("confirmation reader stopped unexpectedly".to_string());
                }
            }
        }
    }

    fn committed(&mut self, commit_id: &str, pushed: bool) {
        if pushed {
            eprintln!("yeet: committed {commit_id} and pushed successfully");
        } else {
            eprintln!("yeet: committed {commit_id} locally (push disabled)");
        }
    }

    fn cancelled(&mut self) {
        eprintln!("yeet: cancelled; changes remain staged");
    }
}

pub fn stdin_is_interactive() -> bool {
    io::stdin().is_terminal()
}
