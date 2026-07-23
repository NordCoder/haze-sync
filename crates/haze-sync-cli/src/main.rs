mod commands;
mod config;
mod config_loader;
mod doctor;
mod doctor_live;
mod http_transport;
mod operations;
mod output;
mod server_api;
mod worktree_api;

// The legacy type remains as a compile-time compatibility boundary only.
// Every executable command path constructs `AuthenticatedHttpClient` instead.
const _: usize = std::mem::size_of::<worktree_api::DeferredWorktreeClient>();

use commands::{AdaptersCommand, CliCommand, HelpTopic, WorktreeCommand};
use config::{CliConfig, OutputFormat};
use config_loader::{load_process_config, split_global_options, ProcessTokenProvider};
use doctor::{DoctorCommand, DoctorMode};
use http_transport::AuthenticatedHttpClient;
use output::CliOutput;
use server_api::ReadCommandMode;
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    emit(run(env::args()))
}

fn run<I, S>(args: I) -> CliOutput
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let raw_args = args
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect::<Vec<_>>();
    let invocation = match split_global_options(raw_args.iter().map(String::as_str)) {
        Ok(invocation) => invocation,
        Err(error) => return CliOutput::usage_error(format!("{error}\n\n{}", usage())),
    };
    let explicit_output = match explicit_output_format(&raw_args) {
        Ok(format) => format,
        Err(error) => {
            return CliOutput::runtime_error(format!("configuration error: {error}"));
        }
    };
    let command = match commands::parse_cli(invocation.command_args.iter().map(String::as_str)) {
        Ok(command) => command,
        Err(error) => {
            return CliOutput::usage_error(format!("{error}\n\n{}", usage()))
                .formatted(explicit_output.unwrap_or(OutputFormat::Human), "unknown");
        }
    };
    let command_name = command.command_name();

    if let CliCommand::Help(topic) = command {
        return CliOutput::success(match topic {
            HelpTopic::Root => usage(),
            HelpTopic::Doctor => doctor::usage().to_owned(),
        })
        .formatted(explicit_output.unwrap_or(OutputFormat::Human), command_name);
    }

    if command_is_offline(&command) {
        let format = explicit_output.unwrap_or(OutputFormat::Human);
        let mut config = CliConfig::default();
        config.output_format = format;
        return render_command(command, config).formatted(format, command_name);
    }

    let config = match load_process_config(&invocation.overrides) {
        Ok(config) => config,
        Err(error) => {
            return CliOutput::runtime_error(format!("configuration error: {error}"))
                .formatted(explicit_output.unwrap_or(OutputFormat::Human), command_name);
        }
    };
    let format = config.output_format;
    render_command(command, config).formatted(format, command_name)
}

fn explicit_output_format(args: &[String]) -> Result<Option<OutputFormat>, config::ConfigError> {
    let mut index = 1;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--output" {
            return args
                .get(index + 1)
                .map(String::as_str)
                .map(OutputFormat::parse)
                .transpose();
        }
        if let Some(value) = argument.strip_prefix("--output=") {
            return OutputFormat::parse(value).map(Some);
        }
        if matches!(
            argument.as_str(),
            "--config" | "--profile" | "--server-url" | "--token-source"
        ) {
            index += 2;
            continue;
        }
        if ["--config=", "--profile=", "--server-url=", "--token-source="]
            .into_iter()
            .any(|prefix| argument.starts_with(prefix))
        {
            index += 1;
            continue;
        }
        break;
    }
    Ok(None)
}

fn command_is_offline(command: &CliCommand) -> bool {
    matches!(
        command,
        CliCommand::Status(commands::StatusCommand {
            mode: ReadCommandMode::Offline,
        }) | CliCommand::Adapters(AdaptersCommand::List {
            mode: ReadCommandMode::Offline,
        }) | CliCommand::Doctor(DoctorCommand {
            mode: DoctorMode::Offline,
        }) | CliCommand::OperationalPlan(_)
    )
}

fn render_command(command: CliCommand, config: CliConfig) -> CliOutput {
    let token_provider = ProcessTokenProvider::new(config.token_source.clone());
    let client = AuthenticatedHttpClient::new(token_provider);

    match command {
        CliCommand::Help(_) => unreachable!("help commands return before configuration loading"),
        CliCommand::Status(command) => {
            server_api::render_status_command(&config, command.mode, &client)
        }
        CliCommand::Adapters(AdaptersCommand::List { mode }) => {
            server_api::render_adapters_command(&config, mode, &client)
        }
        CliCommand::Doctor(command) => match command.mode {
            DoctorMode::Offline => {
                let report = command.build_offline_report();
                CliOutput::success(doctor::render_offline_report(&report))
            }
            DoctorMode::Live => doctor_live::render_live_doctor(&config, &client),
        },
        CliCommand::Worktree(WorktreeCommand::Status) => {
            worktree_api::render_worktree_status(&config, &client)
        }
        CliCommand::Worktree(WorktreeCommand::SyncOnce) => {
            worktree_api::render_worktree_sync_once(&config, &client)
        }
        CliCommand::Preflight => operations::render_preflight(&config, &client),
        CliCommand::OperationalPlan(plan) => operations::render_plan(plan),
    }
}

fn usage() -> String {
    let command_usage = commands::usage();
    let command_list = command_usage
        .strip_prefix("usage: haze-sync <command>\n\n")
        .unwrap_or(command_usage);
    format!(
        "usage: haze-sync [global options] <command>\n\nglobal options:\n  --config <path>         bounded profile configuration file\n  --profile <name>        selected configuration profile\n  --server-url <url>      Server base URL override\n  --output <format>       human, text, or json\n  --token-source <source> none, env:NAME, file:PATH, stdin, or os-secret:service/account\n\n{command_list}"
    )
}

fn emit(output: CliOutput) -> ExitCode {
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
    output.exit_code.into_exit_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::CliExitCode;

    #[test]
    fn help_documents_bounded_configuration_sources() {
        let output = run(["haze-sync", "--help"]);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("--config <path>"));
        assert!(output.stdout.contains("--token-source <source>"));
        assert!(output.stdout.contains("worktree sync-once"));
    }

    #[test]
    fn default_doctor_remains_offline_without_config_loading() {
        let output = run(["haze-sync", "doctor"]);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("doctor mode: offline"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn explicit_offline_status_does_not_require_config_or_token() {
        let output = run([
            "haze-sync",
            "--config",
            "/definitely/not/read/in/offline/mode",
            "status",
            "--offline",
        ]);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("status: offline"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn unconfigured_live_commands_remain_truthful() {
        let status = run(["haze-sync", "status"]);
        assert_eq!(status.exit_code, CliExitCode::Success);
        assert!(status.stdout.contains("status: not_configured"));
        assert!(status.stdout.contains("live server calls: not attempted"));

        let doctor = run(["haze-sync", "doctor", "--live"]);
        assert_eq!(doctor.exit_code, CliExitCode::RuntimeError);
        assert!(doctor.stdout.contains("live checks: not_run"));
        assert!(doctor.stderr.contains("server URL is not configured"));
    }

    #[test]
    fn worktree_commands_require_live_configuration() {
        for arguments in [
            ["haze-sync", "worktree", "status"],
            ["haze-sync", "worktree", "sync-once"],
        ] {
            let output = run(arguments);
            assert_eq!(output.exit_code, CliExitCode::RuntimeError);
            assert!(output.stdout.is_empty());
            assert!(output.stderr.contains("not configured"));
        }
    }

    #[test]
    fn global_option_errors_do_not_echo_values() {
        let private = "redacted-private-value";
        let output = run([
            "haze-sync",
            "--server-url",
            private,
            "--server-url",
            private,
            "status",
        ]);
        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(!output.stderr.contains(private));
    }

    #[test]
    fn config_error_is_runtime_failure_and_secret_safe() {
        let output = run(["haze-sync", "--server-url", "not-a-valid-url", "status"]);
        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stderr.contains("configuration error"));
        assert!(!output.stderr.contains("not-a-valid-url"));
    }

    #[test]
    fn parse_errors_are_safe_and_actionable() {
        let sensitive = "--private-value=redacted-test-value";
        let output = run(["haze-sync", "status", sensitive]);
        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(output.stderr.starts_with("unexpected argument"));
        assert!(output.stderr.contains("usage: haze-sync"));
        assert!(!output.stderr.contains("redacted-test-value"));
    }

    #[test]
    fn doctor_help_still_uses_doctor_specific_usage() {
        let output = run(["haze-sync", "doctor", "--help"]);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output
            .stdout
            .contains("usage: haze-sync doctor [--offline]"));
        assert!(output.stdout.contains("haze-sync doctor --live"));
    }

    #[test]
    fn preflight_requires_live_configuration() {
        let output = run(["haze-sync", "preflight"]);
        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("preflight: not_run"));
        assert!(output.stderr.contains("server URL is not configured"));
    }

    #[test]
    fn plans_support_explicit_stable_json_without_loading_config() {
        let output = run([
            "haze-sync",
            "--config",
            "/definitely/not/read/for/plan",
            "--output",
            "json",
            "recovery",
            "plan",
        ]);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
        assert_eq!(json["schema"], "haze-sync.cli.output.v1");
        assert_eq!(json["command"], "recovery plan");
        assert_eq!(json["ok"], true);
        assert_eq!(json["exit_code"], 0);
        assert!(json["stdout"].as_str().unwrap().contains("writes: none"));
    }

    #[test]
    fn parse_errors_use_json_when_explicitly_selected() {
        let output = run([
            "haze-sync",
            "--output",
            "json",
            "rollout",
            "plan",
            "--force",
        ]);
        assert_eq!(output.exit_code, CliExitCode::UsageError);
        assert!(output.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
        assert_eq!(json["command"], "unknown");
        assert_eq!(json["ok"], false);
        assert_eq!(json["exit_code"], 2);
        assert!(json["stderr"].as_str().unwrap().contains("unexpected argument"));
    }
}
