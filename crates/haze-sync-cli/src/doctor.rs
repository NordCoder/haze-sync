use haze_sync_core::doctor::{
    adapter_token_sanity_check, db_connectivity_check, missing_blob_detection_check,
    object_store_exists_writable_check, AdapterTokenSanityInput, DbConnectivityCheckInput,
    DoctorReport, MissingBlobDetectionInput, ObjectStoreExistsWritableInput,
};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    Doctor(DoctorCommand),
    Help,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CliParseError {
    UnknownCommand,
    UnknownDoctorFlag,
    UnexpectedDoctorArgument,
}

impl fmt::Display for CliParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownCommand => "unknown command",
            Self::UnknownDoctorFlag => "unknown doctor flag",
            Self::UnexpectedDoctorArgument => "unexpected doctor argument",
        };
        formatter.write_str(message)
    }
}

impl Error for CliParseError {}

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
        _command => Err(CliParseError::UnknownCommand),
    }
}

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

#[must_use]
pub const fn usage() -> &'static str {
    "usage: haze-sync doctor [--offline]"
}

fn parse_doctor_args<S>(args: &[S]) -> Result<CliCommand, CliParseError>
where
    S: AsRef<str>,
{
    let mut command = DoctorCommand::default();

    for argument in args {
        match argument.as_ref() {
            "--offline" => command.offline = true,
            value if value.starts_with('-') => return Err(CliParseError::UnknownDoctorFlag),
            _value => return Err(CliParseError::UnexpectedDoctorArgument),
        }
    }

    Ok(CliCommand::Doctor(command))
}
