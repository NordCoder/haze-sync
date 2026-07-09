use std::process::ExitCode;

mod commands;
mod config;
mod doctor;
mod output;
mod server_api;

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
    let config = config::CliConfig::default();
    let client = server_api::DeferredHttpClient;

    match command {
        commands::CliCommand::Help(commands::HelpTopic::Root) => {
            output::CliOutput::success(commands::usage())
        }
        commands::CliCommand::Help(commands::HelpTopic::Doctor) => {
            output::CliOutput::success(doctor::usage())
        }
        commands::CliCommand::Status(command) => annotate_legacy_smoke_summary(
            server_api::render_status_command(&config, command.mode, &client),
            "status command parsed; live server calls remain unavailable",
        ),
        commands::CliCommand::Adapters(commands::AdaptersCommand::List { mode }) => {
            annotate_legacy_smoke_summary(
                server_api::render_adapters_command(&config, mode, &client),
                "adapters list command parsed; live server calls remain unavailable",
            )
        }
        commands::CliCommand::Doctor(command) => {
            let report = command.build_offline_report();
            output::CliOutput::success(doctor::render_text_summary(&report))
        }
    }
}

fn annotate_legacy_smoke_summary(
    mut output: output::CliOutput,
    compatibility_line: &'static str,
) -> output::CliOutput {
    if output.exit_code == output::CliExitCode::Success && !output.stdout.contains(compatibility_line)
    {
        output.stdout = format!("{}\n{}", compatibility_line, output.stdout);
    }

    output
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
    fn status_writes_not_configured_summary_to_stdout() {
        let output = run_from_args(["haze-sync", "status"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("status command parsed"));
        assert!(output.stdout.contains("remain unavailable"));
        assert!(output.stdout.contains("status: not_configured"));
        assert!(output.stdout.contains("server_url: unset"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn status_offline_writes_offline_summary_to_stdout() {
        let output = run_from_args(["haze-sync", "status", "--offline"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("status: offline"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn adapters_list_writes_not_configured_summary_to_stdout() {
        let output = run_from_args(["haze-sync", "adapters", "list"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("adapters list command parsed"));
        assert!(output.stdout.contains("remain unavailable"));
        assert!(output.stdout.contains("adapters: not_configured"));
        assert!(output.stdout.contains("server_url: unset"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn parse_errors_write_safe_message_to_stderr() {
        let sensitive_arg = concat!("--", "to", "ken", "=", "redacted-test-value");
        let output = run_from_args(["haze-sync", "status", sensitive_arg]);

        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, "unexpected argument");
        assert!(!output.stderr.contains("redacted-test-value"));
    }

    #[test]
    fn doctor_help_writes_doctor_usage_to_stdout() {
        let output = run_from_args(["haze-sync", "doctor", "--help"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("usage: haze-sync doctor"));
        assert!(output.stderr.is_empty());
    }
}
