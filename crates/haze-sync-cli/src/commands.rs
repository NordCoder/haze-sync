//! Minimal CLI command model and parser.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliCommand {
    Status,
    Adapters(AdaptersCommand),
}

impl CliCommand {
    #[must_use]
    pub const fn foundation_message(&self) -> &'static str {
        match self {
            Self::Status => "status command parsed; live server calls are not implemented in this foundation",
            Self::Adapters(AdaptersCommand::List) => {
                "adapters list command parsed; live server calls are not implemented in this foundation"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptersCommand {
    List,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliParseError {
    MissingCommand,
    UnknownCommand(String),
    MissingAdaptersCommand,
    UnknownAdaptersCommand(String),
    UnexpectedArgument(String),
}

impl fmt::Display for CliParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("missing command: expected status or adapters"),
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::MissingAdaptersCommand => formatter.write_str("missing adapters command: expected list"),
            Self::UnknownAdaptersCommand(command) => {
                write!(formatter, "unknown adapters command: {command}")
            }
            Self::UnexpectedArgument(argument) => write!(formatter, "unexpected argument: {argument}"),
        }
    }
}

impl std::error::Error for CliParseError {}

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
        return Err(CliParseError::UnexpectedArgument(argument.as_ref().to_owned()));
    }

    Ok(())
}
