//! Haze Sync command-line foundation binary.
//!
//! This binary only parses safe operational-introspection commands. It does not
//! start network services, call live servers, connect to databases, contact
//! providers, read credentials, or perform synchronization.

use std::process::ExitCode;

mod commands;

fn main() -> ExitCode {
    match commands::parse_cli(std::env::args()) {
        Ok(command) => {
            println!("{}", command.foundation_message());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
