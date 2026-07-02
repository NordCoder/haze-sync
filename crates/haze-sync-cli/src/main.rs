use std::process::ExitCode;

mod commands;
mod doctor;

fn main() -> ExitCode {
    let args = std::env::args().collect::<Vec<_>>();

    if args.get(1).is_some_and(|command| command == "doctor") {
        return match doctor::parse_cli_args(args.iter().skip(1)) {
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
        };
    }

    match commands::parse_cli(args.iter()) {
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
