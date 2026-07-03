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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_doctor_report_is_reachable_and_secret_safe() {
        let report = DoctorCommand::default().build_offline_report();
        let json = serde_json::to_string(&report).expect("doctor report should serialize");

        assert_eq!(report.summary.total_checks, 4);
        assert!(json.contains("db_connectivity"));
        assert!(json.contains("object_store_exists_writable"));
        assert!(json.contains("missing_blobs"));
        assert!(json.contains("adapter_token_sanity"));
        assert_no_secret_leaks(&json);
    }

    #[test]
    fn rendered_doctor_summary_is_secret_safe() {
        let report = DoctorCommand::default().build_offline_report();
        let summary = render_text_summary(&report);

        assert!(summary.contains("doctor summary"));
        assert!(summary.contains("total: 4"));
        assert_no_secret_leaks(&summary);
    }

    #[test]
    fn unsupported_live_or_repair_args_are_rejected_safely() {
        assert_eq!(
            parse_cli_args(["doctor", "--repair"]).unwrap_err(),
            CliParseError::UnknownDoctorFlag
        );
        assert_eq!(
            parse_cli_args(["doctor", "provider-call"]).unwrap_err(),
            CliParseError::UnexpectedDoctorArgument
        );
    }

    fn assert_no_secret_leaks(output: &str) {
        for forbidden in [
            "credential",
            "oauth",
            "secret",
            "database_url",
            "db_url",
            "provider_payload",
            "postgres://",
            "/srv/",
            "C:\\",
            "object_store_root",
            "backtrace",
            "stack",
        ] {
            assert!(
                !output.contains(forbidden),
                "doctor output leaked forbidden marker {forbidden}: {output}"
            );
        }
    }
}
