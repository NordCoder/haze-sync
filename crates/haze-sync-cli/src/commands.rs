//! Minimal CLI command model and parser.
//!
//! The parser is intentionally dependency-free and side-effect-free. It builds a
//! command model for future operational wiring without opening network
//! connections, reading credentials, contacting providers, or mutating state.

use std::fmt;

/// Parsed top-level CLI command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliCommand {
    /// `haze-sync status` foundation command.
    Status,
    /// `haze-sync adapters ...` foundation command group.
    Adapters(AdaptersCommand),
}

impl CliCommand {
    /// Human-readable foundation message printed by the placeholder binary.
    #[must_use]
    pub const fn foundation_message(&self) -> &'static str {
        match self {
            Self::Status => {
                "status command parsed; live server calls are not implemented in this foundation"
            }
            Self::Adapters(AdaptersCommand::List) => {
                "adapters list command parsed; live server calls are not implemented in this foundation"
            }
        }
    }
}

/// Parsed adapters subcommand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptersCommand {
    /// `haze-sync adapters list` foundation command.
    List,
}

/// Safe CLI parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliParseError {
    /// No top-level command was provided.
    MissingCommand,
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
            Self::MissingCommand => {
                formatter.write_str("missing command: expected status or adapters")
            }
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
    let command = next_argument(&mut args).ok_or(CliParseError::MissingCommand)?;

    match command.as_str() {
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
            command.foundation_message(),
            "status command parsed; live server calls are not implemented in this foundation"
        );
    }

    #[test]
    fn adapters_list_command_parses() {
        let command = parse_cli(["haze-sync", "adapters", "list"]).unwrap();

        assert_eq!(command, CliCommand::Adapters(AdaptersCommand::List));
        assert_eq!(
            command.foundation_message(),
            "adapters list command parsed; live server calls are not implemented in this foundation"
        );
    }

    #[test]
    fn parser_rejects_unscoped_network_arguments() {
        let error =
            parse_cli(["haze-sync", "status", "--server", "https://example.test"]).unwrap_err();

        assert_eq!(error, CliParseError::UnexpectedArgument("--server".to_owned()));
    }

    #[test]
    fn parser_errors_are_safe() {
        let error = parse_cli(["haze-sync", "tokens"]).unwrap_err().to_string();

        for forbidden in ["token_hash", "oauth", "secret", "database_url", "/srv/", "backtrace"] {
            assert!(!error.contains(forbidden));
        }
    }
}
