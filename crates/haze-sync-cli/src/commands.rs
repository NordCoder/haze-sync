//! Minimal CLI command model and parser.
//!
//! The parser is intentionally dependency-free and side-effect-free. Parse
//! errors avoid echoing raw arguments because output is commonly copied into
//! logs, tickets, and chat.

use crate::{
    doctor::{self, DoctorCliCommand, DoctorCommand, DoctorParseError},
    operations::OperationalPlan,
    server_api::ReadCommandMode,
};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliCommand {
    Help(HelpTopic),
    Status(StatusCommand),
    Adapters(AdaptersCommand),
    Doctor(DoctorCommand),
    Worktree(WorktreeCommand),
    Preflight,
    OperationalPlan(OperationalPlan),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpTopic {
    Root,
    Doctor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusCommand {
    pub mode: ReadCommandMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptersCommand {
    List { mode: ReadCommandMode },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeCommand {
    Status,
    SyncOnce,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliParseError {
    UnknownCommand,
    MissingAdaptersCommand,
    UnknownAdaptersCommand,
    MissingWorktreeCommand,
    UnknownWorktreeCommand,
    MissingPlanCommand,
    UnknownPlanCommand,
    UnexpectedArgument,
    Doctor(DoctorParseError),
}

impl fmt::Display for CliParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand => formatter.write_str("unknown command"),
            Self::MissingAdaptersCommand => {
                formatter.write_str("missing adapters command: expected list")
            }
            Self::UnknownAdaptersCommand => formatter.write_str("unknown adapters command"),
            Self::MissingWorktreeCommand => {
                formatter.write_str("missing worktree command: expected status or sync-once")
            }
            Self::UnknownWorktreeCommand => formatter.write_str("unknown worktree command"),
            Self::MissingPlanCommand => formatter.write_str("missing plan command: expected plan"),
            Self::UnknownPlanCommand => formatter.write_str("unknown plan command"),
            Self::UnexpectedArgument => formatter.write_str("unexpected argument"),
            Self::Doctor(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for CliParseError {}

impl CliCommand {
    #[must_use]
    pub const fn command_name(&self) -> &'static str {
        match self {
            Self::Help(_) => "help",
            Self::Status(_) => "status",
            Self::Adapters(_) => "adapters list",
            Self::Doctor(_) => "doctor",
            Self::Worktree(WorktreeCommand::Status) => "worktree status",
            Self::Worktree(WorktreeCommand::SyncOnce) => "worktree sync-once",
            Self::Preflight => "preflight",
            Self::OperationalPlan(plan) => plan.command_name(),
        }
    }
}

#[must_use]
pub const fn usage() -> &'static str {
    "usage: haze-sync <command>\n\ncommands:\n  status [--offline]        read-only server status summary\n  adapters list [--offline] read-only adapter summary\n  doctor [--offline]        read-only offline doctor summary\n  doctor --live             read-only Server health/readiness/status doctor\n  worktree status           read hosted Worktree runtime status\n  worktree sync-once        request one bounded server-owned DryRun cycle
  preflight                 live read-only operational readiness gate
  bootstrap plan            dry-run trusted-source bootstrap checklist
  recovery plan             dry-run backup/restore recovery checklist
  rollout plan              dry-run staged adapter rollout checklist"
}

pub fn parse_cli<I, S>(args: I) -> Result<CliCommand, CliParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let _program_name = args.next();
    let Some(command) = next_argument(&mut args) else {
        return Ok(CliCommand::Help(HelpTopic::Root));
    };
    match command.as_str() {
        "--help" | "-h" | "help" => {
            reject_trailing(args)?;
            Ok(CliCommand::Help(HelpTopic::Root))
        }
        "status" => parse_status_command(args),
        "adapters" => parse_adapters_command(args),
        "doctor" => parse_doctor_command(args),
        "worktree" => parse_worktree_command(args),
        "preflight" => {
            reject_trailing(args)?;
            Ok(CliCommand::Preflight)
        },
        "bootstrap" => parse_plan_command(args, OperationalPlan::Bootstrap),
        "recovery" => parse_plan_command(args, OperationalPlan::Recovery),
        "rollout" => parse_plan_command(args, OperationalPlan::Rollout),
        _ => Err(CliParseError::UnknownCommand),
    }
}

fn parse_status_command<I, S>(args: I) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    Ok(CliCommand::Status(StatusCommand {
        mode: parse_read_mode(args)?,
    }))
}

fn parse_adapters_command<I, S>(mut args: I) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let command = next_argument(&mut args).ok_or(CliParseError::MissingAdaptersCommand)?;
    match command.as_str() {
        "list" => Ok(CliCommand::Adapters(AdaptersCommand::List {
            mode: parse_read_mode(args)?,
        })),
        _ => Err(CliParseError::UnknownAdaptersCommand),
    }
}

fn parse_doctor_command<I, S>(args: I) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    match doctor::parse_doctor_args(args).map_err(CliParseError::Doctor)? {
        DoctorCliCommand::Doctor(command) => Ok(CliCommand::Doctor(command)),
        DoctorCliCommand::Help => Ok(CliCommand::Help(HelpTopic::Doctor)),
    }
}

fn parse_worktree_command<I, S>(mut args: I) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let command = next_argument(&mut args).ok_or(CliParseError::MissingWorktreeCommand)?;
    let command = match command.as_str() {
        "status" => WorktreeCommand::Status,
        "sync-once" => WorktreeCommand::SyncOnce,
        _ => return Err(CliParseError::UnknownWorktreeCommand),
    };
    reject_trailing(args)?;
    Ok(CliCommand::Worktree(command))
}

fn parse_plan_command<I, S>(mut args: I, plan: OperationalPlan) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let command = next_argument(&mut args).ok_or(CliParseError::MissingPlanCommand)?;
    if command != "plan" {
        return Err(CliParseError::UnknownPlanCommand);
    }
    reject_trailing(args)?;
    Ok(CliCommand::OperationalPlan(plan))
}

fn parse_read_mode<I, S>(args: I) -> Result<ReadCommandMode, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut mode = ReadCommandMode::Auto;
    for argument in args {
        match argument.as_ref() {
            "--offline" => mode = ReadCommandMode::Offline,
            _ => return Err(CliParseError::UnexpectedArgument),
        }
    }
    Ok(mode)
}

fn next_argument<I, S>(args: &mut I) -> Option<String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next().map(|argument| argument.as_ref().to_owned())
}

fn reject_trailing<I, S>(mut args: I) -> Result<(), CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if args.next().is_some() {
        return Err(CliParseError::UnexpectedArgument);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::DoctorMode;

    #[test]
    fn status_command_parses() {
        assert_eq!(
            parse_cli(["haze-sync", "status"]).unwrap(),
            CliCommand::Status(StatusCommand {
                mode: ReadCommandMode::Auto
            })
        );
    }

    #[test]
    fn status_offline_command_parses() {
        assert_eq!(
            parse_cli(["haze-sync", "status", "--offline"]).unwrap(),
            CliCommand::Status(StatusCommand {
                mode: ReadCommandMode::Offline
            })
        );
    }

    #[test]
    fn adapters_list_command_parses() {
        assert_eq!(
            parse_cli(["haze-sync", "adapters", "list"]).unwrap(),
            CliCommand::Adapters(AdaptersCommand::List {
                mode: ReadCommandMode::Auto
            })
        );
    }

    #[test]
    fn adapters_list_offline_command_parses() {
        assert_eq!(
            parse_cli(["haze-sync", "adapters", "list", "--offline"]).unwrap(),
            CliCommand::Adapters(AdaptersCommand::List {
                mode: ReadCommandMode::Offline
            })
        );
    }

    #[test]
    fn doctor_modes_parse_through_top_level_model() {
        assert_eq!(
            parse_cli(["haze-sync", "doctor", "--offline"]).unwrap(),
            CliCommand::Doctor(DoctorCommand {
                mode: DoctorMode::Offline
            })
        );
        assert_eq!(
            parse_cli(["haze-sync", "doctor", "--live"]).unwrap(),
            CliCommand::Doctor(DoctorCommand {
                mode: DoctorMode::Live
            })
        );
    }

    #[test]
    fn conflicting_doctor_modes_are_rejected() {
        assert_eq!(
            parse_cli(["haze-sync", "doctor", "--offline", "--live"]).unwrap_err(),
            CliParseError::Doctor(DoctorParseError::ConflictingDoctorModes)
        );
    }

    #[test]
    fn help_and_empty_invocation_render_usage() {
        assert_eq!(
            parse_cli(["haze-sync"]).unwrap(),
            CliCommand::Help(HelpTopic::Root)
        );
        assert_eq!(
            parse_cli(["haze-sync", "--help"]).unwrap(),
            CliCommand::Help(HelpTopic::Root)
        );
        assert_eq!(
            parse_cli(["haze-sync", "doctor", "--help"]).unwrap(),
            CliCommand::Help(HelpTopic::Doctor)
        );
        assert!(usage().contains("doctor [--offline]"));
        assert!(usage().contains("doctor --live"));
        assert!(usage().contains("read-only"));
    }

    #[test]
    fn parser_rejects_unscoped_network_arguments() {
        assert_eq!(
            parse_cli(["haze-sync", "status", "--server", "https://example.test"]).unwrap_err(),
            CliParseError::UnexpectedArgument
        );
    }

    #[test]
    fn worktree_commands_parse_and_appear_in_usage() {
        assert_eq!(
            parse_cli(["haze-sync", "worktree", "status"]).unwrap(),
            CliCommand::Worktree(WorktreeCommand::Status)
        );
        assert_eq!(
            parse_cli(["haze-sync", "worktree", "sync-once"]).unwrap(),
            CliCommand::Worktree(WorktreeCommand::SyncOnce)
        );
        assert!(usage().contains("worktree status"));
        assert!(usage().contains("worktree sync-once"));
        assert!(usage().contains("bounded server-owned DryRun cycle"));
    }

    #[test]
    fn worktree_rejects_missing_unknown_and_control_arguments_without_echo() {
        assert_eq!(
            parse_cli(["haze-sync", "worktree"]).unwrap_err(),
            CliParseError::MissingWorktreeCommand
        );
        assert_eq!(
            parse_cli(["haze-sync", "worktree", "private-command"]).unwrap_err(),
            CliParseError::UnknownWorktreeCommand
        );
        for private in [
            "--force",
            "--path=private",
            "--budget=999",
            "--ticket=secret",
            "--generation=42",
            "{\"mode\":\"bidirectional\"}",
        ] {
            let error = parse_cli(["haze-sync", "worktree", "sync-once", private])
                .unwrap_err()
                .to_string();
            assert_eq!(error, "unexpected argument");
            assert!(!error.contains(private));
        }
    }

    #[test]
    fn operational_commands_parse_and_are_documented() {
        assert_eq!(parse_cli(["haze-sync", "preflight"]).unwrap(), CliCommand::Preflight);
        assert_eq!(
            parse_cli(["haze-sync", "bootstrap", "plan"]).unwrap(),
            CliCommand::OperationalPlan(OperationalPlan::Bootstrap)
        );
        assert_eq!(
            parse_cli(["haze-sync", "recovery", "plan"]).unwrap(),
            CliCommand::OperationalPlan(OperationalPlan::Recovery)
        );
        assert_eq!(
            parse_cli(["haze-sync", "rollout", "plan"]).unwrap(),
            CliCommand::OperationalPlan(OperationalPlan::Rollout)
        );
        assert!(usage().contains("preflight"));
        assert!(usage().contains("bootstrap plan"));
        assert!(usage().contains("recovery plan"));
        assert!(usage().contains("rollout plan"));
    }

    #[test]
    fn operational_plan_parser_is_bounded() {
        assert_eq!(
            parse_cli(["haze-sync", "bootstrap"]).unwrap_err(),
            CliParseError::MissingPlanCommand
        );
        assert_eq!(
            parse_cli(["haze-sync", "recovery", "apply"]).unwrap_err(),
            CliParseError::UnknownPlanCommand
        );
        assert_eq!(
            parse_cli(["haze-sync", "rollout", "plan", "--force"]).unwrap_err(),
            CliParseError::UnexpectedArgument
        );
    }

    #[test]
    fn parser_errors_do_not_echo_arguments() {
        let private_command = "private-command";
        let private_flag = "--private-value=redacted-test-value";
        let examples = [
            parse_cli(["haze-sync", private_command])
                .unwrap_err()
                .to_string(),
            parse_cli(["haze-sync", "status", private_flag])
                .unwrap_err()
                .to_string(),
            parse_cli(["haze-sync", "adapters", private_command])
                .unwrap_err()
                .to_string(),
            parse_cli(["haze-sync", "doctor", private_flag])
                .unwrap_err()
                .to_string(),
            parse_cli(["haze-sync", "worktree", private_flag])
                .unwrap_err()
                .to_string(),
        ];
        for error in examples {
            assert!(!error.contains(private_command));
            assert!(!error.contains(private_flag));
            assert!(!error.contains("redacted-test-value"));
        }
    }
}
