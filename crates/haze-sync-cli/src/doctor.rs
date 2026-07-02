//! Offline-safe doctor command scaffolding.
//!
//! This module parses a minimal `doctor` command and builds a passive report
//! from Core doctor models. It does not read configuration files, connect to a
//! database, call providers, repair state, delete files, or expose secrets.

use haze_sync_core::doctor::{
    adapter_token_sanity_check, db_connectivity_check, missing_blob_detection_check,
    object_store_exists_writable_check, AdapterTokenSanityInput, DbConnectivityCheckInput,
    DoctorReport, MissingBlobDetectionInput, ObjectStoreExistsWritableInput,
};
use std::{error::Error, fmt};

/// Parsed top-level CLI command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    Doctor(DoctorCommand),
    Help,
}

/// Parsed `doctor` command options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoctorCommand {
    pub offline: bool,
}

impl Default for DoctorCommand {
    fn default() -> Self {
        Self { offline: true }
    }
}

impl DoctorCommand {
    /// Builds the default passive doctor report without runtime dependency calls.
    #[must_use]
    pub fn build_offline_report(self) -> DoctorReport {
        DoctorReport::from_results(vec![
            db_connectivity_check(DbConnectivityCheckInput::offline(false)),
            object_store_exists_writable_check(ObjectStoreExistsWritableInput::offline(false)),
            missing_blob_detection_check(MissingBlobDetectionInput::new(0, Vec::new(), 5)),
            adapter_token_sanity_check(AdapterTokenSanityInput::new(Vec::new())),
        ])
    }
}

/// CLI parse error with no runtime or secret payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliParseError {
    UnknownCommand(String),
    UnknownDoctorFlag(String),
    UnexpectedDoctorArgument(String),
}

impl fmt::Display for CliParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
            Self::UnknownDoctorFlag(flag) => write!(formatter, "unknown doctor flag: {flag}"),
            Self::UnexpectedDoctorArgument(argument) => {
                write!(formatter, "unexpected doctor argument: {argument}")
            }
        }
    }
}

impl Error for CliParseError {}

/// Parses command arguments after the executable name.
pub fn parse_cli_args<I, S>(args: I) -> Result<CliCommand, CliParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let collected = args.into_iter().collect::<Vec<_>>();
    let Some(first) = collected.first() else {
        return Ok(CliCommand::Help);
    };

    match first.as_ref() {
        "--help" | "-h" | "help" => Ok(CliCommand::Help),
        "doctor" => parse_doctor_args(&collected[1..]),
        command => Err(CliParseError::UnknownCommand(command.to_owned())),
    }
}

/// Parses process environment arguments after skipping the executable name.
pub fn parse_env_args() -> Result<CliCommand, CliParseError> {
    parse_cli_args(std::env::args().skip(1))
}

/// Renders a safe one-line doctor summary for the placeholder CLI.
#[must_use]
pub fn render_text_summary(report: &DoctorReport) -> String {
    format!(
        "doctor summary: {:?} (total: {}, ok: {}, warnings: {}, failed: {}, skipped: {})",
        report.summary.status,
        report.summary.total_checks,
        report.summary.ok_count,
        report.summary.warning_count,
        report.summary.failed_count,
        report.summary.skipped_count
    )
    .to_lowercase()
}

/// Usage text for the safe doctor scaffold.
#[must_use]
pub const fn usage() -> &'static str {
    "usage: haze-sync-cli doctor [--offline]"
}

fn parse_doctor_args<S>(args: &[S]) -> Result<CliCommand, CliParseError>
where
    S: AsRef<str>,
{
    let mut command = DoctorCommand::default();

    for argument in args {
        match argument.as_ref() {
            "--offline" => command.offline = true,
            "--repair" | "--apply" | "--fix" => {
                return Err(CliParseError::UnknownDoctorFlag(argument.as_ref().to_owned()));
            }
            value if value.starts_with('-') => {
                return Err(CliParseError::UnknownDoctorFlag(value.to_owned()));
            }
            value => return Err(CliParseError::UnexpectedDoctorArgument(value.to_owned())),
        }
    }

    Ok(CliCommand::Doctor(command))
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_core::doctor::DoctorCheckStatus;

    #[test]
    fn doctor_command_parses_with_offline_default() {
        assert_eq!(
            parse_cli_args(["doctor"]).expect("doctor command should parse"),
            CliCommand::Doctor(DoctorCommand { offline: true })
        );
        assert_eq!(
            parse_cli_args(["doctor", "--offline"]).expect("doctor command should parse"),
            CliCommand::Doctor(DoctorCommand { offline: true })
        );
    }

    #[test]
    fn doctor_command_rejects_repair_or_mutation_flags() {
        assert_eq!(
            parse_cli_args(["doctor", "--repair"]).expect_err("repair is out of scope"),
            CliParseError::UnknownDoctorFlag("--repair".to_owned())
        );
        assert_eq!(
            parse_cli_args(["doctor", "--apply"]).expect_err("apply is out of scope"),
            CliParseError::UnknownDoctorFlag("--apply".to_owned())
        );
    }

    #[test]
    fn offline_doctor_report_builds_without_runtime_calls() {
        let report = DoctorCommand::default().build_offline_report();

        assert_eq!(report.summary.total_checks, 4);
        assert_eq!(report.summary.status, DoctorCheckStatus::Warning);
        assert_eq!(
            report
                .checks
                .iter()
                .map(|check| check.check_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "adapter_token_sanity",
                "db_connectivity",
                "missing_blobs",
                "object_store_exists_writable",
            ]
        );
    }
}
