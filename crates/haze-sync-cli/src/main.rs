//! Haze Sync command-line placeholder binary.
//!
//! This CLI exposes safe command scaffolding only. It does not start network
//! services, connect to providers, read credentials, repair state, or perform
//! synchronization.

mod doctor;

use std::process::ExitCode;

fn main() -> ExitCode {
    match doctor::parse_env_args() {
        Ok(doctor::CliCommand::Doctor(command)) => {
            let report = command.build_offline_report();
            println!("{}", doctor::render_text_summary(&report));
            ExitCode::SUCCESS
        }
        Ok(doctor::CliCommand::Help) => {
            println!("{}", doctor::usage());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
