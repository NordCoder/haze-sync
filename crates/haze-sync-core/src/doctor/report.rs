use super::types::{DoctorCheckResult, DoctorCheckStatus};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Aggregate doctor report summary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DoctorReportSummary {
    pub status: DoctorCheckStatus,
    pub total_checks: u64,
    pub ok_count: u64,
    pub warning_count: u64,
    pub failed_count: u64,
    pub skipped_count: u64,
    pub not_run_count: u64,
    pub placeholder_count: u64,
}

impl DoctorReportSummary {
    /// Derives a summary from deterministic check results.
    ///
    /// Aggregate precedence is failed, warning, placeholder, not-run, skipped,
    /// then ok. An empty result set is not healthy evidence and therefore has
    /// `not_run` status.
    #[must_use]
    pub fn from_results(results: &[DoctorCheckResult]) -> Self {
        let mut ok_count = 0_u64;
        let mut warning_count = 0_u64;
        let mut failed_count = 0_u64;
        let mut skipped_count = 0_u64;
        let mut not_run_count = 0_u64;
        let mut placeholder_count = 0_u64;
        for result in results {
            match result.status() {
                DoctorCheckStatus::Ok => ok_count = ok_count.saturating_add(1),
                DoctorCheckStatus::Warning => warning_count = warning_count.saturating_add(1),
                DoctorCheckStatus::Failed => failed_count = failed_count.saturating_add(1),
                DoctorCheckStatus::Skipped => skipped_count = skipped_count.saturating_add(1),
                DoctorCheckStatus::NotRun => not_run_count = not_run_count.saturating_add(1),
                DoctorCheckStatus::Placeholder => {
                    placeholder_count = placeholder_count.saturating_add(1)
                }
            }
        }
        let status = if failed_count > 0 {
            DoctorCheckStatus::Failed
        } else if warning_count > 0 {
            DoctorCheckStatus::Warning
        } else if placeholder_count > 0 {
            DoctorCheckStatus::Placeholder
        } else if not_run_count > 0 || results.is_empty() {
            DoctorCheckStatus::NotRun
        } else if skipped_count > 0 {
            DoctorCheckStatus::Skipped
        } else {
            DoctorCheckStatus::Ok
        };
        Self {
            status,
            total_checks: usize_to_u64(results.len()),
            ok_count,
            warning_count,
            failed_count,
            skipped_count,
            not_run_count,
            placeholder_count,
        }
    }
}

/// Deterministically ordered doctor report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorReport {
    pub summary: DoctorReportSummary,
    pub checks: Vec<DoctorCheckResult>,
}

impl DoctorReport {
    /// Creates a doctor report with checks ordered by stable check identifier.
    #[must_use]
    pub fn from_results(mut checks: Vec<DoctorCheckResult>) -> Self {
        checks.sort_by_key(|result| result.check_id().as_str());
        let summary = DoctorReportSummary::from_results(&checks);
        Self { summary, checks }
    }

    #[must_use]
    pub const fn summary(&self) -> &DoctorReportSummary {
        &self.summary
    }

    #[must_use]
    pub fn checks(&self) -> &[DoctorCheckResult] {
        &self.checks
    }

    fn validate(&self) -> Result<(), &'static str> {
        for check in &self.checks {
            check.validate()?;
        }
        if !self.checks.windows(2).all(|window| {
            window[0].check_id().as_str() <= window[1].check_id().as_str()
        }) {
            return Err("doctor report checks are not deterministically ordered");
        }
        if self.summary != DoctorReportSummary::from_results(&self.checks) {
            return Err("doctor report summary does not match check results");
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct DoctorReportWireRef<'a> {
    summary: &'a DoctorReportSummary,
    checks: &'a [DoctorCheckResult],
}

impl Serialize for DoctorReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        DoctorReportWireRef {
            summary: &self.summary,
            checks: &self.checks,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DoctorReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DoctorReportWire {
            summary: DoctorReportSummary,
            checks: Vec<DoctorCheckResult>,
        }

        let wire = DoctorReportWire::deserialize(deserializer)?;
        let report = Self::from_results(wire.checks);
        if report.summary != wire.summary {
            return Err(serde::de::Error::custom(
                "doctor report summary does not match check results",
            ));
        }
        report.validate().map_err(serde::de::Error::custom)?;
        Ok(report)
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
