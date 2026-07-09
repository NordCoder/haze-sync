//! Minimal CLI command model and parser.
//!
//! The parser is intentionally dependency-free and side-effect-free. It builds a
//! command model for future operational wiring without opening network
//! connections, reading operator inputs, contacting providers, or mutating
//! state. Parse errors intentionally avoid echoing raw arguments because CLI
//! output is commonly copied into logs, tickets, and chat.

use crate::doctor::{self, DoctorCliCommand, DoctorCommand, DoctorParseError};
use std::fmt;

/// Parsed top-level CLI command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliCommand {
    /// `haze-sync --help` / `haze-sync help` or command-specific help.
    Help(HelpTopic),
    /// `haze-sync status` scaffold command.
    Status,
    /// `haze-sync adapters ...` scaffold command group.
    Adapters(AdaptersCommand),
    /// `haze-sync doctor [--offline]` offline diagnostic command.
    Doctor(DoctorCommand),
}

/// Help topic requested by the operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpTopic {
    /// Root CLI usage.
    Root,
    /// Doctor command usage.
    Doctor,
}

/// Parsed adapters subcommand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptersCommand {
    /// `haze-sync adapters list` scaffold command.
    List,
}

/// Safe CLI parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliParseError {
    /// Unknown top-level command.
    UnknownCommand,
    /// No adapters subcommand was provided.
    MissingAdaptersCommand,
    /// Unknown adapters subcommand.
    UnknownAdaptersCommand,
    /// Extra argument was provided after a complete command.
    UnexpectedArgument,
    /// Doctor command parse failure.
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
            Self::UnexpectedArgument => formatter.write_str("unexpected argument"),
            Self::Doctor(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for CliParseError {}

#[must_use]
pub const fn usage() -> &'static str {
    "usage: haze-sync <command>\n\ncommands:\n  status             read-only placeholder status summary\n  adapters list      read-only placeholder adapter summary\n  doctor [--offline] read-only offline doctor summary"
}

/// Parse process arguments into the minimal command model.
///
/// The first item is treated as the program name and ignored.
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
        "status" => {
            reject_trailing(args)?;
            Ok(CliCommand::Status)
        }
        "adapters" => parse_adapters_command(args),
        "doctor" => parse_doctor_command(args),
        _ => Err(CliParseError::UnknownCommand),
    }
}

fn parse_adapters_command<I, S>(mut args: I) -> Result<CliCommand, CliParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let command = next_argument(&mut args).ok_or(CliParseError::MissingAdaptersCommand)?;

    match command.as_str() {
        "list" => {
            reject_trailing(args)?;
            Ok(CliCommand::Adapters(AdaptersCommand::List))
        }
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

    #[test]
    fn status_command_parses() {
        let command = parse_cli(["haze-sync", "status"]).unwrap();

        assert_eq!(command, CliCommand::Status);
    }

    #[test]
    fn adapters_list_command_parses() {
        let command = parse_cli(["haze-sync", "adapters", "list"]).unwrap();

        assert_eq!(command, CliCommand::Adapters(AdaptersCommand::List));
    }

    #[test]
    fn doctor_command_parses_through_top_level_model() {
        let command = parse_cli(["haze-sync", "doctor", "--offline"]).unwrap();

        assert_eq!(command, CliCommand::Doctor(DoctorCommand { offline: true }));
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
        assert!(usage().contains("read-only"));
    }

    #[test]
    fn parser_rejects_unscoped_network_arguments() {
        let error =
            parse_cli(["haze-sync", "status", "--server", "https://example.test"]).unwrap_err();

        assert_eq!(error, CliParseError::UnexpectedArgument);
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
        ];

        for error in examples {
            assert!(!error.contains(private_command));
            assert!(!error.contains(private_flag));
            assert!(!error.contains("redacted-test-value"));
        }
    }
}
