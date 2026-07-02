use std::process::ExitCode;

mod commands;
mod doctor;

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
