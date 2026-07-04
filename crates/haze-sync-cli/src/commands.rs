//! Minimal CLI command model and parser.
//!
//! The parser is intentionally dependency-free and side-effect-free. It builds a
//! command model for future operational wiring without opening network
//! connections, reading credentials, contacting providers, or mutating state.

use std::fmt;

/// Parsed top-level CLI command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliCommand {
    /// `haze-sync --help` / `haze-sync help`.
    Help,
    /// `haze-sync status` scaffold command.
    Status,
    /// `haze-sync adapters ...` scaffold command group.
    Adapters(AdaptersCommand),
}

impl CliCommand {
    /// Human-readable summary printed by the current scaffold binary.
    #[must_use]
    pub const fn summary_message(&self) -> &'static str {
        match self {
            Self::Help => usage(),
            Self::Status => "status command parsed; live server calls remain unavailable",
            Self::Adapters(AdaptersCommand::List) => {
                "adapters list command parsed; live server calls remain unavailable"
            }
        }
    }
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
    UnknownCommand(String),
    /// No adapters subcommand was provided.
    MissingAdaptersCommand,
    /// Unknown adapters subcommand.
    UnknownAdaptersCommand(String),
    /// Extra argument was provided after a complete command.
    UnexpectedArgument(String),
}

impl fmt::Display for CliParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::MissingAdaptersCommand => {
                formatter.write_str("missing adapters command: expected list")
            }
            Self::UnknownAdaptersCommand(command) => {
                write!(formatter, "unknown adapters command: {command}")
            }
            Self::UnexpectedArgument(argument) => {
                write!(formatter, "unexpected argument: {argument}")
            }
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
        return Ok(CliCommand::Help);
    };

    match command.as_str() {
        "--help" | "-h" | "help" => {
            reject_trailing(args)?;
            Ok(CliCommand::Help)
        }
        "status" => {
            reject_trailing(args)?;
            Ok(CliCommand::Status)
        }
        "adapters" => parse_adapters_command(args),
        _ => Err(CliParseError::UnknownCommand(command)),
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
        _ => Err(CliParseError::UnknownAdaptersCommand(command)),
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
    if let Some(argument) = args.next() {
        return Err(CliParseError::UnexpectedArgument(
            argument.as_ref().to_owned(),
        ));
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
        assert_eq!(
            command.summary_message(),
            "status command parsed; live server calls remain unavailable"
        );
    }

    #[test]
    fn adapters_list_command_parses() {
        let command = parse_cli(["haze-sync", "adapters", "list"]).unwrap();

        assert_eq!(command, CliCommand::Adapters(AdaptersCommand::List));
        assert_eq!(
            command.summary_message(),
            "adapters list command parsed; live server calls remain unavailable"
        );
    }

    #[test]
    fn help_and_empty_invocation_render_usage() {
        assert_eq!(parse_cli(["haze-sync"]).unwrap(), CliCommand::Help);
        assert_eq!(
            parse_cli(["haze-sync", "--help"]).unwrap(),
            CliCommand::Help
        );
        assert!(usage().contains("doctor [--offline]"));
        assert!(usage().contains("read-only"));
    }

    #[test]
    fn parser_rejects_unscoped_network_arguments() {
        let error =
            parse_cli(["haze-sync", "status", "--server", "https://example.test"]).unwrap_err();

        assert_eq!(
            error,
            CliParseError::UnexpectedArgument("--server".to_owned())
        );
    }

    #[test]
    fn parser_errors_are_safe() {
        let error = parse_cli(["haze-sync", "tokens"]).unwrap_err().to_string();

        for forbidden in [
            "token_hash",
            "oauth",
            "secret",
            "database_url",
            "/srv/",
            "backtrace",
        ] {
            assert!(!error.contains(forbidden));
        }
    }
}
