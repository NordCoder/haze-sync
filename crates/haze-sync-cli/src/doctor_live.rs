#![allow(dead_code)]
//! Contract-backed live doctor aggregation over accepted Server surfaces.
//!
//! No dedicated doctor endpoint exists. Live doctor therefore reads only the
//! accepted public `/health`, `/ready`, and `/v1/admin/status` surfaces, maps
//! those sanitized summaries into Core doctor models, and marks unsupported
//! checks as skipped instead of reaching into databases, object stores, or
//! providers directly.

use crate::{
    config::{CliConfig, ServerUrl},
    doctor,
    output::{CliExitCode, CliOutput},
    server_api::{
        DeferredHttpClient, ServerReadClient, ServerReadError, ServerStatus, StatusSummary,
    },
};
use haze_sync_core::doctor::{
    db_connectivity_check, object_store_exists_writable_check, AdapterTokenSanityDetails,
    DbConnectivityCheckInput, DbConnectivityDetails, DoctorCheckDetails, DoctorCheckId,
    DoctorCheckResult, DoctorCheckStatus, DoctorReport, MissingBlobDetectionDetails,
    ObjectStoreExistsWritableDetails, ObjectStoreExistsWritableInput,
};

/// Accepted public Server surface used by live doctor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DoctorEndpoint {
    Health,
    Readiness,
    AdminStatus,
}

impl DoctorEndpoint {
    #[must_use]
    pub const fn method(self) -> &'static str {
        "GET"
    }

    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Health => "/health",
            Self::Readiness => "/ready",
            Self::AdminStatus => "/v1/admin/status",
        }
    }
}

/// Sanitized `/health` result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthSummary {
    pub process_alive: bool,
}

/// Sanitized overall `/ready` status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessOverallStatus {
    Ready,
    NotReady,
}

impl ReadinessOverallStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotReady => "not_ready",
        }
    }
}

/// Sanitized readiness state for one accepted dependency component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessComponentState {
    Ready,
    NotReady,
    Disabled,
    Unknown,
}

/// Sanitized subset of the accepted `/ready` response.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadinessSummary {
    pub overall: ReadinessOverallStatus,
    pub database: ReadinessComponentState,
    pub object_store: ReadinessComponentState,
}

/// Read-only client boundary used by live doctor.
pub trait DoctorReadClient {
    fn fetch_health(&self, server_url: &ServerUrl) -> Result<HealthSummary, ServerReadError>;

    fn fetch_readiness(&self, server_url: &ServerUrl)
        -> Result<ReadinessSummary, ServerReadError>;

    fn fetch_status(&self, server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError>;
}

impl DoctorReadClient for DeferredHttpClient {
    fn fetch_health(&self, _server_url: &ServerUrl) -> Result<HealthSummary, ServerReadError> {
        Err(ServerReadError::ServerUnavailable)
    }

    fn fetch_readiness(
        &self,
        _server_url: &ServerUrl,
    ) -> Result<ReadinessSummary, ServerReadError> {
        Err(ServerReadError::ServerUnavailable)
    }

    fn fetch_status(&self, server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
        ServerReadClient::fetch_status(self, server_url)
    }
}

/// Execute live doctor through accepted public Server surfaces only.
#[must_use]
pub fn render_live_doctor<C>(config: &CliConfig, client: &C) -> CliOutput
where
    C: DoctorReadClient,
{
    let Some(server_url) = config.server_url.as_ref() else {
        return CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout: "doctor mode: live\nlive checks: not_run\nreason: server_not_configured"
                .to_owned(),
            stderr: "doctor live mode could not start: server URL is not configured".to_owned(),
        };
    };

    let health = client.fetch_health(server_url);
    let readiness = client.fetch_readiness(server_url);
    let status = client.fetch_status(server_url);
    let report = build_core_report(readiness.as_ref().ok());
    let stdout = render_live_summary(&health, &readiness, &status, &report);
    let failures = collect_surface_failures(&health, &readiness, &status);

    if failures.is_empty() {
        CliOutput::success(stdout)
    } else {
        CliOutput {
            exit_code: CliExitCode::RuntimeError,
            stdout,
            stderr: format!("doctor live checks incomplete: {}", failures.join(", ")),
        }
    }
}

fn build_core_report(readiness: Option<&ReadinessSummary>) -> DoctorReport {
    let database = readiness.map_or_else(skipped_database_check, |summary| {
        database_check(summary.database)
    });
    let object_store = readiness.map_or_else(skipped_object_store_check, |summary| {
        object_store_check(summary.object_store)
    });

    DoctorReport::from_results(vec![
        database,
        object_store,
        skipped_missing_blob_check(),
        skipped_adapter_token_check(),
    ])
}

fn database_check(state: ReadinessComponentState) -> DoctorCheckResult {
    match state {
        ReadinessComponentState::Ready => db_connectivity_check(DbConnectivityCheckInput {
            metadata_configured: true,
            live_check_enabled: true,
            connectivity_verified: Some(true),
        }),
        ReadinessComponentState::NotReady => db_connectivity_check(DbConnectivityCheckInput {
            metadata_configured: true,
            live_check_enabled: true,
            connectivity_verified: Some(false),
        }),
        ReadinessComponentState::Disabled => {
            db_connectivity_check(DbConnectivityCheckInput::offline(false))
        }
        ReadinessComponentState::Unknown => skipped_database_check(),
    }
}

fn object_store_check(state: ReadinessComponentState) -> DoctorCheckResult {
    match state {
        ReadinessComponentState::Ready => {
            object_store_exists_writable_check(ObjectStoreExistsWritableInput {
                configured: true,
                exists: Some(true),
                writable: Some(true),
            })
        }
        ReadinessComponentState::NotReady => DoctorCheckResult::new(
            DoctorCheckId::ObjectStoreExistsWritable,
            DoctorCheckStatus::Failed,
            "object store readiness check failed",
            DoctorCheckDetails::ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails {
                configured: true,
                exists: None,
                writable: None,
            }),
        ),
        ReadinessComponentState::Disabled => {
            object_store_exists_writable_check(ObjectStoreExistsWritableInput::offline(false))
        }
        ReadinessComponentState::Unknown => skipped_object_store_check(),
    }
}

fn skipped_database_check() -> DoctorCheckResult {
    DoctorCheckResult::new(
        DoctorCheckId::DbConnectivity,
        DoctorCheckStatus::Skipped,
        "database readiness check was not run",
        DoctorCheckDetails::DbConnectivity(DbConnectivityDetails {
            metadata_configured: false,
            live_check_performed: false,
            connectivity_verified: None,
        }),
    )
}

fn skipped_object_store_check() -> DoctorCheckResult {
    DoctorCheckResult::new(
        DoctorCheckId::ObjectStoreExistsWritable,
        DoctorCheckStatus::Skipped,
        "object store readiness check was not run",
        DoctorCheckDetails::ObjectStoreExistsWritable(ObjectStoreExistsWritableDetails {
            configured: false,
            exists: None,
            writable: None,
        }),
    )
}

fn skipped_missing_blob_check() -> DoctorCheckResult {
    DoctorCheckResult::new(
        DoctorCheckId::MissingBlobs,
        DoctorCheckStatus::Skipped,
        "missing blob check not run: no accepted public diagnostic surface",
        DoctorCheckDetails::MissingBlobs(MissingBlobDetectionDetails {
            input_count: 0,
            missing_count: 0,
            sample_hashes: Vec::new(),
        }),
    )
}

fn skipped_adapter_token_check() -> DoctorCheckResult {
    DoctorCheckResult::new(
        DoctorCheckId::AdapterTokenSanity,
        DoctorCheckStatus::Skipped,
        "adapter token sanity check not run: no accepted public diagnostic surface",
        DoctorCheckDetails::AdapterTokenSanity(AdapterTokenSanityDetails {
            enabled_adapter_count: 0,
            missing_token_hash_count: 0,
            invalid_role_count: 0,
        }),
    )
}

fn render_live_summary(
    health: &Result<HealthSummary, ServerReadError>,
    readiness: &Result<ReadinessSummary, ServerReadError>,
    status: &Result<StatusSummary, ServerReadError>,
    report: &DoctorReport,
) -> String {
    let health_line = match health {
        Ok(summary) if summary.process_alive => "health: ok".to_owned(),
        Ok(_) => "health: not_ready".to_owned(),
        Err(error) => format!("health: error_{}", safe_error_code(*error)),
    };
    let readiness_line = match readiness {
        Ok(summary) => format!("readiness: {}", summary.overall.as_str()),
        Err(error) => format!("readiness: error_{}", safe_error_code(*error)),
    };
    let status_line = match status {
        Ok(summary) => format!("server status: {}", summary.server_status.as_str()),
        Err(error) => format!("server status: error_{}", safe_error_code(*error)),
    };

    format!(
        "doctor mode: live\n{health_line}\n{readiness_line}\n{status_line}\n{}",
        doctor::render_detailed_report(report)
    )
}

fn collect_surface_failures(
    health: &Result<HealthSummary, ServerReadError>,
    readiness: &Result<ReadinessSummary, ServerReadError>,
    status: &Result<StatusSummary, ServerReadError>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if let Err(error) = health {
        failures.push(format!("health={}", safe_error_code(*error)));
    }
    if let Err(error) = readiness {
        failures.push(format!("readiness={}", safe_error_code(*error)));
    }
    if let Err(error) = status {
        failures.push(format!("status={}", safe_error_code(*error)));
    }
    failures
}

const fn safe_error_code(error: ServerReadError) -> &'static str {
    match error {
        ServerReadError::NotConfigured => "not_configured",
        ServerReadError::Offline => "offline",
        ServerReadError::Unauthorized => "unauthorized",
        ServerReadError::Forbidden => "forbidden",
        ServerReadError::ServerUnavailable => "server_unavailable",
        ServerReadError::NotReady => "not_ready",
        ServerReadError::InvalidResponse => "invalid_response",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{OutputFormat, ProfileName, TokenSource};
    use crate::server_api::{DependencyReadinessState, PauseStatusSummary};

    #[derive(Clone)]
    struct FakeDoctorClient {
        health: Result<HealthSummary, ServerReadError>,
        readiness: Result<ReadinessSummary, ServerReadError>,
        status: Result<StatusSummary, ServerReadError>,
    }

    impl DoctorReadClient for FakeDoctorClient {
        fn fetch_health(&self, _server_url: &ServerUrl) -> Result<HealthSummary, ServerReadError> {
            self.health.clone()
        }

        fn fetch_readiness(
            &self,
            _server_url: &ServerUrl,
        ) -> Result<ReadinessSummary, ServerReadError> {
            self.readiness.clone()
        }

        fn fetch_status(&self, _server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
            self.status.clone()
        }
    }

    fn configured() -> CliConfig {
        CliConfig::new(
            ProfileName::parse("ops").unwrap(),
            Some(ServerUrl::parse("https://sync.example.test").unwrap()),
            OutputFormat::Human,
            TokenSource::None,
        )
    }

    fn ready_status() -> StatusSummary {
        StatusSummary {
            server_status: ServerStatus::Ready,
            db_readiness_state: DependencyReadinessState::Ready,
            object_store_readiness_state: DependencyReadinessState::Ready,
            last_operation_sequence: Some(42),
            adapter_count: Some(1),
            pause: PauseStatusSummary {
                supported: true,
                active: Some(false),
            },
        }
    }

    fn ready_client() -> FakeDoctorClient {
        FakeDoctorClient {
            health: Ok(HealthSummary {
                process_alive: true,
            }),
            readiness: Ok(ReadinessSummary {
                overall: ReadinessOverallStatus::Ready,
                database: ReadinessComponentState::Ready,
                object_store: ReadinessComponentState::Ready,
            }),
            status: Ok(ready_status()),
        }
    }

    #[test]
    fn accepted_live_doctor_endpoints_are_get_only() {
        for endpoint in [
            DoctorEndpoint::Health,
            DoctorEndpoint::Readiness,
            DoctorEndpoint::AdminStatus,
        ] {
            assert_eq!(endpoint.method(), "GET");
        }
        assert_eq!(DoctorEndpoint::Health.path(), "/health");
        assert_eq!(DoctorEndpoint::Readiness.path(), "/ready");
        assert_eq!(DoctorEndpoint::AdminStatus.path(), "/v1/admin/status");
    }

    #[test]
    fn live_doctor_without_server_config_is_not_run() {
        let output = render_live_doctor(&CliConfig::default(), &ready_client());

        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("doctor mode: live"));
        assert!(output.stdout.contains("live checks: not_run"));
        assert!(output.stderr.contains("server URL is not configured"));
    }

    #[test]
    fn live_doctor_maps_readiness_to_core_report_and_skips_unavailable_checks() {
        let output = render_live_doctor(&configured(), &ready_client());

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("health: ok"));
        assert!(output.stdout.contains("readiness: ready"));
        assert!(output.stdout.contains("server status: ready"));
        assert!(output.stdout.contains("ok: 2"));
        assert!(output.stdout.contains("skipped: 2"));
        assert!(output.stdout.contains("missing blob check not run"));
        assert!(output.stdout.contains("adapter token sanity check not run"));
        assert!(output.stderr.is_empty());
        assert_no_sensitive_markers(&output.stdout);
    }

    #[test]
    fn not_ready_dependencies_are_reported_as_failed_checks() {
        let mut client = ready_client();
        client.readiness = Ok(ReadinessSummary {
            overall: ReadinessOverallStatus::NotReady,
            database: ReadinessComponentState::NotReady,
            object_store: ReadinessComponentState::NotReady,
        });
        let output = render_live_doctor(&configured(), &client);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("readiness: not_ready"));
        assert!(output.stdout.contains("failed: 2"));
        assert_no_sensitive_markers(&output.stdout);
    }

    #[test]
    fn surface_errors_keep_partial_report_and_safe_runtime_error() {
        let mut client = ready_client();
        client.status = Err(ServerReadError::Unauthorized);
        let output = render_live_doctor(&configured(), &client);

        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stdout.contains("health: ok"));
        assert!(output.stdout.contains("server status: error_unauthorized"));
        assert_eq!(
            output.stderr,
            "doctor live checks incomplete: status=unauthorized"
        );
        assert_no_sensitive_markers(&output.stdout);
        assert_no_sensitive_markers(&output.stderr);
    }

    fn assert_no_sensitive_markers(output: &str) {
        for marker in [
            "route_test_token",
            "token_hash",
            "oauth",
            "secret",
            "postgres://",
            "database_url",
            "object_store_root",
            "external_cursor_json",
            "/srv/",
            "C:\\",
            "stack",
            "backtrace",
        ] {
            assert!(
                !output.contains(marker),
                "doctor output leaked forbidden marker {marker}: {output}"
            );
        }
    }
}
