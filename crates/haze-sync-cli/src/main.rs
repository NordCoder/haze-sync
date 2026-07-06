use std::process::ExitCode;

mod commands;
mod doctor;
mod output;

fn main() -> ExitCode {
    let output = run_from_args(std::env::args());
    write_output(&output);
    output.exit_code.into_exit_code()
}

fn run_from_args<I, S>(args: I) -> output::CliOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    match commands::parse_cli(args) {
        Ok(command) => render_command(command),
        Err(error) => output::CliOutput::usage_error(error.to_string()),
    }
}

fn render_command(command: commands::CliCommand) -> output::CliOutput {
    match command {
        commands::CliCommand::Help(commands::HelpTopic::Root) => {
            output::CliOutput::success(commands::usage())
        }
        commands::CliCommand::Help(commands::HelpTopic::Doctor) => {
            output::CliOutput::success(doctor::usage())
        }
        commands::CliCommand::Status | commands::CliCommand::Adapters(_) => {
            let summary = command
                .placeholder_summary()
                .expect("placeholder command must have a scaffold summary");
            output::CliOutput::success(summary)
        }
        commands::CliCommand::Doctor(command) => {
            let report = command.build_offline_report();
            output::CliOutput::success(doctor::render_text_summary(&report))
        }
    }
}

fn write_output(output: &output::CliOutput) {
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }

    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::CliExitCode;

    #[test]
    fn status_writes_summary_to_stdout() {
        let output = run_from_args(["haze-sync", "status"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert_eq!(
            output.stdout,
            "status command parsed; live server calls remain unavailable"
        );
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn parse_errors_write_safe_message_to_stderr() {
        let output = run_from_args(["haze-sync", "status", "--token=supersecret"]);

        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, "unexpected argument");
        assert!(!output.stderr.contains("supersecret"));
    }

    #[test]
    fn doctor_help_writes_doctor_usage_to_stdout() {
        let output = run_from_args(["haze-sync", "doctor", "--help"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("usage: haze-sync doctor"));
        assert!(output.stderr.is_empty());
    }
}
