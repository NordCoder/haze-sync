use std::process::ExitCode;

mod commands;
mod config;
mod doctor;
mod doctor_live;
mod output;
mod server_api;
mod worktree_api;

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
    let worktree_client = worktree_api::DeferredWorktreeClient;

    match command {
        commands::CliCommand::Help(commands::HelpTopic::Root) => {
            output::CliOutput::success(commands::usage())
        }
        commands::CliCommand::Help(commands::HelpTopic::Doctor) => {
            output::CliOutput::success(doctor::usage())
        }
        commands::CliCommand::Status(command) => annotate_unconfigured_placeholder(
            server_api::render_status_command(&config, command.mode, &client),
            "status: not_configured",
            "status command parsed; live server calls remain unavailable",
        ),
        commands::CliCommand::Adapters(commands::AdaptersCommand::List { mode }) => {
            annotate_unconfigured_placeholder(
                server_api::render_adapters_command(&config, mode, &client),
                "adapters: not_configured",
                "adapters list command parsed; live server calls remain unavailable",
            )
        }
        commands::CliCommand::Doctor(command) => match command.mode {
            doctor::DoctorMode::Offline => {
                let report = command.build_offline_report();
                output::CliOutput::success(doctor::render_offline_report(&report))
            }
            doctor::DoctorMode::Live => doctor_live::render_live_doctor(&config, &client),
        },
        commands::CliCommand::Worktree(commands::WorktreeCommand::Status) => {
            worktree_api::render_worktree_status(&config, &worktree_client)
        }
        commands::CliCommand::Worktree(commands::WorktreeCommand::SyncOnce) => {
            worktree_api::render_worktree_sync_once(&config, &worktree_client)
        }
    }
}

fn annotate_unconfigured_placeholder(
    mut output: output::CliOutput,
    placeholder_marker: &'static str,
    compatibility_line: &'static str,
) -> output::CliOutput {
    if output.exit_code == output::CliExitCode::Success
        && output.stdout.contains(placeholder_marker)
        && !output.stdout.contains(compatibility_line)
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
        assert!(!output.stdout.contains("remain unavailable"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn live_success_is_not_annotated_as_unavailable() {
        let output = output::CliOutput::success("server status: ready");
        let output = annotate_unconfigured_placeholder(
            output,
            "status: not_configured",
            "status command parsed; live server calls remain unavailable",
        );

        assert_eq!(output.stdout, "server status: ready");
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
    fn doctor_defaults_to_offline_summary() {
        let output = run_from_args(["haze-sync", "doctor"]);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("doctor mode: offline"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stdout.contains("doctor summary"));
        assert!(output.stdout.contains("total: 4"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn live_doctor_without_config_is_not_run_and_non_zero() {
        let output = run_from_args(["haze-sync", "doctor", "--live"]);

        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("doctor mode: live"));
        assert!(output.stdout.contains("live checks: not_run"));
        assert!(output.stderr.contains("server URL is not configured"));
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
        assert!(output
            .stdout
            .contains("usage: haze-sync doctor [--offline]"));
        assert!(output.stdout.contains("haze-sync doctor --live"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn worktree_commands_require_live_configuration_and_do_not_fake_success() {
        for args in [
            ["haze-sync", "worktree", "status"],
            ["haze-sync", "worktree", "sync-once"],
        ] {
            let output = run_from_args(args);
            assert_eq!(output.exit_code, CliExitCode::RuntimeError);
            assert!(output.stdout.is_empty());
            assert!(output.stderr.contains("not configured"));
        }
    }

    #[test]
    fn worktree_parse_errors_do_not_echo_sensitive_arguments() {
        let sensitive_arg = "--token=redacted-test-value";
        let output = run_from_args(["haze-sync", "worktree", "sync-once", sensitive_arg]);

        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, "unexpected argument");
        assert!(!output.stderr.contains("redacted-test-value"));
    }
}
