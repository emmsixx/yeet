// SPDX-License-Identifier: GPL-3.0-or-later

use std::process::ExitCode;

use clap::Parser;
use yeet_cli::cli::Cli;
use yeet_cli::{RunOutcome, run};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(RunOutcome::ConfigChanged(messages)) => {
            for message in messages {
                println!("{message}");
            }
            ExitCode::SUCCESS
        }
        Ok(RunOutcome::ConfigShown(output)) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Ok(RunOutcome::Workflow(outcome)) => {
            if matches!(outcome, yeet_cli::application::WorkflowOutcome::Cancelled) {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("yeet: {error}");
            ExitCode::FAILURE
        }
    }
}
