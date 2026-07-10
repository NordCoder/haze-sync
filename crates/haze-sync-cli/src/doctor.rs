use haze_sync_core::doctor::{
    adapter_token_sanity_check, db_connectivity_check, missing_blob_detection_check,
    object_store_exists_writable_check, AdapterTokenSanityInput, DbConnectivityCheckInput,
    DoctorCheckStatus, DoctorReport, MissingBlobDetectionInput, ObjectStoreExistsWritableInput,
};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DoctorCliCommand {
    Doctor(DoctorCommand),
    Help,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DoctorMode {
    #[default]
    Offline,
    Live,
}

impl DoctorMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Live => "live",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DoctorCommand {
    pub mode: DoctorMode,
}

impl DoctorCommand {
    #[must_use]
    pub fn build_offline_report(self) -> DoctorReport {
        debug_assert_eq!(self.mode, DoctorMode::Offline);
        DoctorReport::from_results(vec![
            db_connectivity_check(DbConnectivityCheckInput::offline(false)),
            object_store_exists_writable_check(ObjectStoreExistsWritableInput::offline(false)),
            missing_blob_detection_check(MissingBlobDetectionInput::new(0, Vec::new(), 5)),
            adapter_token_sanity_check(AdapterTokenSanityInput::new(Vec::new())),
        ])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoctorParseError {
    UnknownDoctorFlag,
    UnexpectedDoctorArgument,
    ConflictingDoctorModes,
}

impl fmt::Display for DoctorParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownDoctorFlag => "unknown doctor flag",
            Self::UnexpectedDoctorArgument => "unexpected doctor argument",
            Self::ConflictingDoctorModes => "conflicting doctor modes",
        };
        formatter.write_str(message)
    }
}

impl Error for DoctorParseError {}

pub fn parse_doctor_args<I, S>(args: I) -> Result<DoctorCliCommand, DoctorParseError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let mut selected_mode = None;

    while let Some(argument) = next_argument(&mut args) {
        match argument.as_str() {
            "--help" | "-h" | "help" => {
                reject_trailing(args)?;
                return Ok(DoctorCliCommand::Help);
            }
            "--offline" => select_mode(&mut selected_mode, DoctorMode::Offline)?,
            "--live" => select_mode(&mut selected_mode, DoctorMode::Live)?,
            value if value.starts_with('-') => return Err(DoctorParseError::UnknownDoctorFlag),
            _value => return Err(DoctorParseError::UnexpectedDoctorArgument),
        }
    }

    Ok(DoctorCliCommand::Doctor(DoctorCommand {
        mode: selected_mode.unwrap_or_default(),
    }))
}

fn select_mode(
    selected_mode: &mut Option<DoctorMode>,
    requested_mode: DoctorMode,
) -> Result<(), DoctorParseError> {
    if selected_mode.is_some_and(|mode| mode != requested_mode) {
        return Err(DoctorParseError::ConflictingDoctorModes);
    }
    *selected_mode = Some(requested_mode);
    Ok(())
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
pub fn render_detailed_report(report: &DoctorReport) -> String {
    let mut lines = vec![render_text_summary(report)];
    for check in &report.checks {
        lines.push(format!(
            "check {}: {} - {}",
            check.check_id.as_str(),
            status_label(check.status),
            check.message
        ));
    }
    lines.join("\n")
}

#[must_use]
pub fn render_offline_report(report: &DoctorReport) -> String {
    format!(
        "doctor mode: offline\nlive server calls: not attempted\n{}",
        render_detailed_report(report)
    )
}

const fn status_label(status: DoctorCheckStatus) -> &'static str {
    match status {
        DoctorCheckStatus::Ok => "ok",
        DoctorCheckStatus::Warning => "warning",
        DoctorCheckStatus::Failed => "failed",
        DoctorCheckStatus::Skipped => "skipped",
    }
}

#[must_use]
pub const fn usage() -> &'static str {
    "usage: haze-sync doctor [--offline]\n       haze-sync doctor --live\n\nmodes:\n  --offline  read-only offline summary; no network calls\n  --live     read-only aggregation of accepted Server health/readiness/status surfaces\n\nrepair, direct database/provider checks, and destructive actions are unavailable"
}

fn next_argument<I, S>(args: &mut I) -> Option<String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next().map(|argument| argument.as_ref().to_owned())
}

fn reject_trailing<I, S>(mut args: I) -> Result<(), DoctorParseError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if args.next().is_some() {
        return Err(DoctorParseError::UnexpectedDoctorArgument);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_core::doctor::DoctorCheckId;

    #[test]
    fn offline_doctor_report_is_reachable_and_sensitive_safe() {
        let report = DoctorCommand::default().build_offline_report();

        assert_eq!(report.summary.total_checks, 4);
        assert!(report
            .checks
            .iter()
            .any(|check| check.check_id == DoctorCheckId::DbConnectivity));
        assert!(report
            .checks
            .iter()
            .any(|check| check.check_id == DoctorCheckId::ObjectStoreExistsWritable));
        assert!(report
            .checks
            .iter()
            .any(|check| check.check_id == DoctorCheckId::MissingBlobs));
        assert!(report
            .checks
            .iter()
            .any(|check| check.check_id == DoctorCheckId::AdapterTokenSanity));
        for check in &report.checks {
            assert_no_sensitive_leaks(&check.message);
        }
    }

    #[test]
    fn rendered_offline_doctor_report_is_labeled_and_sensitive_safe() {
        let report = DoctorCommand::default().build_offline_report();
        let summary = render_offline_report(&report);

        assert!(summary.contains("doctor mode: offline"));
        assert!(summary.contains("live server calls: not attempted"));
        assert!(summary.contains("doctor summary"));
        assert!(summary.contains("total: 4"));
        assert_no_sensitive_leaks(&summary);
    }

    #[test]
    fn explicit_live_and_offline_modes_parse() {
        assert_eq!(
            parse_doctor_args(["--live"]).unwrap(),
            DoctorCliCommand::Doctor(DoctorCommand {
                mode: DoctorMode::Live,
            })
        );
        assert_eq!(
            parse_doctor_args(["--offline"]).unwrap(),
            DoctorCliCommand::Doctor(DoctorCommand {
                mode: DoctorMode::Offline,
            })
        );
        assert_eq!(
            parse_doctor_args(std::iter::empty::<&str>()).unwrap(),
            DoctorCliCommand::Doctor(DoctorCommand::default())
        );
    }

    #[test]
    fn conflicting_modes_and_unsupported_repair_are_rejected_safely() {
        assert_eq!(
            parse_doctor_args(["--live", "--offline"]).unwrap_err(),
            DoctorParseError::ConflictingDoctorModes
        );
        assert_eq!(
            parse_doctor_args(["--repair"]).unwrap_err(),
            DoctorParseError::UnknownDoctorFlag
        );
        assert_eq!(
            parse_doctor_args(["provider-call"]).unwrap_err(),
            DoctorParseError::UnexpectedDoctorArgument
        );
    }

    #[test]
    fn doctor_help_is_supported_and_usage_stays_safe() {
        assert_eq!(
            parse_doctor_args(["--help"]).unwrap(),
            DoctorCliCommand::Help
        );
        assert!(usage().contains("usage: haze-sync doctor [--offline]"));
        assert!(usage().contains("haze-sync doctor --live"));
        assert!(usage().contains("read-only offline summary"));
        assert_no_sensitive_leaks(usage());
    }

    #[test]
    fn doctor_help_rejects_trailing_arguments_safely() {
        let private_arg = "private-command";
        let error = parse_doctor_args(["--help", private_arg]).unwrap_err();

        assert_eq!(error, DoctorParseError::UnexpectedDoctorArgument);
        assert_no_sensitive_leaks(&error.to_string());
        assert!(!error.to_string().contains(private_arg));
    }

    fn assert_no_sensitive_leaks(output: &str) {
        for forbidden in [
            concat!("cred", "ential"),
            concat!("oa", "uth"),
            concat!("se", "cret"),
            concat!("database", "_url"),
            concat!("db", "_url"),
            concat!("provider", "_payload"),
            concat!("post", "gres", "://"),
            concat!("/", "srv", "/"),
            concat!("C", ":", "\\"),
            concat!("object_store", "_root"),
            concat!("back", "trace"),
            concat!("sta", "ck"),
        ] {
            assert!(
                !output.contains(forbidden),
                "doctor output leaked forbidden marker {forbidden}: {output}"
            );
        }
    }
}
