//! Remote Worktree operator client boundary and deterministic rendering.
//!
//! The CLI submits requests to Server only. It does not host runtime work,
//! inspect local state, poll for completion, or expose private runtime details.

use crate::{
    config::{CliConfig, ServerUrl},
    output::CliOutput,
};
use haze_sync_api::dto::worktree::{
    WorktreeConfiguredMode, WorktreeHostLifecycle, WorktreeManualAvailability, WorktreeReadiness,
    WorktreeReadinessReason, WorktreeStatusResponse, WorktreeSyncOnceRequest,
    WorktreeSyncOnceResponse, WorktreeSyncOnceSubmissionStatus,
};

pub const WORKTREE_STATUS_PATH: &str = "/v1/admin/worktree/status";
pub const WORKTREE_SYNC_ONCE_PATH: &str = "/v1/admin/worktree/sync-once";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorktreeStatusRequest;

impl WorktreeStatusRequest {
    #[must_use]
    pub const fn method(self) -> &'static str {
        "GET"
    }

    #[must_use]
    pub const fn path(self) -> &'static str {
        WORKTREE_STATUS_PATH
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorktreeSyncRequest {
    pub body: WorktreeSyncOnceRequest,
}

impl WorktreeSyncRequest {
    #[must_use]
    pub const fn method(self) -> &'static str {
        "POST"
    }

    #[must_use]
    pub const fn path(self) -> &'static str {
        WORKTREE_SYNC_ONCE_PATH
    }

    #[must_use]
    pub const fn body_json(self) -> &'static str {
        "{}"
    }
}

pub trait WorktreeClient {
    fn fetch_worktree_status(
        &self,
        server_url: &ServerUrl,
        request: WorktreeStatusRequest,
    ) -> Result<(u16, WorktreeStatusResponse), WorktreeClientError>;

    fn submit_worktree_sync_once(
        &self,
        server_url: &ServerUrl,
        request: WorktreeSyncRequest,
    ) -> Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DeferredWorktreeClient;

impl WorktreeClient for DeferredWorktreeClient {
    fn fetch_worktree_status(
        &self,
        _server_url: &ServerUrl,
        _request: WorktreeStatusRequest,
    ) -> Result<(u16, WorktreeStatusResponse), WorktreeClientError> {
        Err(WorktreeClientError::ServerUnavailable)
    }

    fn submit_worktree_sync_once(
        &self,
        _server_url: &ServerUrl,
        _request: WorktreeSyncRequest,
    ) -> Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError> {
        Err(WorktreeClientError::ServerUnavailable)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeClientError {
    NotConfigured,
    Unauthorized,
    Forbidden,
    Busy,
    Failed,
    ServerUnavailable,
    InvalidResponse,
}

impl WorktreeClientError {
    fn from_error_http_status(status: u16) -> Option<Self> {
        match status {
            401 => Some(Self::Unauthorized),
            403 => Some(Self::Forbidden),
            500 | 503 => Some(Self::ServerUnavailable),
            _ => None,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotConfigured => {
                "not configured: set a server URL before running Worktree operator commands"
            }
            Self::Unauthorized => {
                "auth error: admin token is missing or invalid; no token value was printed"
            }
            Self::Forbidden => "forbidden: configured token cannot operate the Worktree runtime",
            Self::Busy => "worktree sync-once: busy; no additional cycle was submitted",
            Self::Failed => "worktree sync-once: failed; Server rejected the bounded cycle request",
            Self::ServerUnavailable => {
                "server unavailable: Worktree operator request could not be completed safely"
            }
            Self::InvalidResponse => {
                "invalid response: Server Worktree response did not match the accepted contract"
            }
        }
    }

    fn into_output(self) -> CliOutput {
        CliOutput::runtime_error(self.message())
    }
}

#[must_use]
pub fn render_worktree_status<C>(config: &CliConfig, client: &C) -> CliOutput
where
    C: WorktreeClient,
{
    let Some(server_url) = config.server_url.as_ref() else {
        return WorktreeClientError::NotConfigured.into_output();
    };

    let request = WorktreeStatusRequest;
    let _request_contract = (request.method(), request.path());
    match client.fetch_worktree_status(server_url, request) {
        Ok((200, response)) => CliOutput::success(render_status(response)),
        Ok((status, _)) => WorktreeClientError::from_error_http_status(status)
            .unwrap_or(WorktreeClientError::InvalidResponse)
            .into_output(),
        Err(error) => error.into_output(),
    }
}

#[must_use]
pub fn render_worktree_sync_once<C>(config: &CliConfig, client: &C) -> CliOutput
where
    C: WorktreeClient,
{
    let Some(server_url) = config.server_url.as_ref() else {
        return WorktreeClientError::NotConfigured.into_output();
    };

    let request = WorktreeSyncRequest::default();
    let _request_contract = (request.method(), request.path(), request.body_json());
    match client.submit_worktree_sync_once(server_url, request) {
        Ok((status, response)) => classify_sync_response(status, response),
        Err(error) => error.into_output(),
    }
}

fn classify_sync_response(http_status: u16, response: WorktreeSyncOnceResponse) -> CliOutput {
    use WorktreeSyncOnceSubmissionStatus as Body;

    match (http_status, response.status) {
        (202, Body::Accepted) => CliOutput::success(
            "worktree sync-once: accepted\ncycle: submitted or queued\ncompletion: not awaited",
        ),
        (409, Body::Busy) => WorktreeClientError::Busy.into_output(),
        (503, Body::NotStarted | Body::Cancelling | Body::Shutdown | Body::Unavailable) => {
            CliOutput::runtime_error(format!(
                "worktree sync-once: {}\ncycle: not submitted",
                submission_status(response.status)
            ))
        }
        (500, Body::Failed) => WorktreeClientError::Failed.into_output(),
        (401 | 403, _) => WorktreeClientError::from_error_http_status(http_status)
            .expect("matched authentication status")
            .into_output(),
        _ => WorktreeClientError::InvalidResponse.into_output(),
    }
}

fn render_status(response: WorktreeStatusResponse) -> String {
    format!(
        "worktree configured_mode: {}\nworktree lifecycle: {}\nworktree readiness: {}\nworktree readiness_reason: {}\ncycles completed: {}\ncycles failed: {}\ncycle in_progress: {}\npending watcher_hints: {}\nmanual availability: {}",
        configured_mode(response.configured_mode),
        lifecycle(response.host_lifecycle),
        readiness(response.readiness),
        readiness_reason(response.readiness_reason),
        response.cycles_completed,
        response.cycles_failed,
        response.cycle_in_progress,
        response.pending_watcher_hints,
        manual_availability(response.manual_availability),
    )
}

const fn configured_mode(value: WorktreeConfiguredMode) -> &'static str {
    match value {
        WorktreeConfiguredMode::Disabled => "disabled",
        WorktreeConfiguredMode::ReadOnly => "read_only",
        WorktreeConfiguredMode::ImportOnly => "import_only",
        WorktreeConfiguredMode::ExportOnly => "export_only",
        WorktreeConfiguredMode::Bidirectional => "bidirectional",
        WorktreeConfiguredMode::DryRun => "dry_run",
    }
}

const fn lifecycle(value: WorktreeHostLifecycle) -> &'static str {
    match value {
        WorktreeHostLifecycle::Disabled => "disabled",
        WorktreeHostLifecycle::Starting => "starting",
        WorktreeHostLifecycle::Running => "running",
        WorktreeHostLifecycle::Cancelling => "cancelling",
        WorktreeHostLifecycle::Shutdown => "shutdown",
        WorktreeHostLifecycle::Failed => "failed",
    }
}

const fn readiness(value: WorktreeReadiness) -> &'static str {
    match value {
        WorktreeReadiness::Ready => "ready",
        WorktreeReadiness::NotReady => "not_ready",
    }
}

const fn readiness_reason(value: WorktreeReadinessReason) -> &'static str {
    match value {
        WorktreeReadinessReason::DisabledInert => "disabled_inert",
        WorktreeReadinessReason::Running => "running",
        WorktreeReadinessReason::Starting => "starting",
        WorktreeReadinessReason::Cancelling => "cancelling",
        WorktreeReadinessReason::Shutdown => "shutdown",
        WorktreeReadinessReason::Failed => "failed",
    }
}

const fn manual_availability(value: WorktreeManualAvailability) -> &'static str {
    match value {
        WorktreeManualAvailability::Available => "available",
        WorktreeManualAvailability::Busy => "busy",
        WorktreeManualAvailability::NotStarted => "not_started",
        WorktreeManualAvailability::Cancelling => "cancelling",
        WorktreeManualAvailability::Shutdown => "shutdown",
        WorktreeManualAvailability::Unavailable => "unavailable",
        WorktreeManualAvailability::Failed => "failed",
    }
}

const fn submission_status(value: WorktreeSyncOnceSubmissionStatus) -> &'static str {
    match value {
        WorktreeSyncOnceSubmissionStatus::Accepted => "accepted",
        WorktreeSyncOnceSubmissionStatus::Busy => "busy",
        WorktreeSyncOnceSubmissionStatus::NotStarted => "not_started",
        WorktreeSyncOnceSubmissionStatus::Cancelling => "cancelling",
        WorktreeSyncOnceSubmissionStatus::Shutdown => "shutdown",
        WorktreeSyncOnceSubmissionStatus::Unavailable => "unavailable",
        WorktreeSyncOnceSubmissionStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{OutputFormat, ProfileName, TokenSource},
        output::CliExitCode,
    };
    use haze_sync_api::dto::worktree::WorktreeStatusSafeParts;
    use std::cell::RefCell;

    struct FakeClient {
        status: Result<(u16, WorktreeStatusResponse), WorktreeClientError>,
        sync: Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError>,
        requests: RefCell<Vec<(&'static str, &'static str, &'static str)>>,
    }

    impl WorktreeClient for FakeClient {
        fn fetch_worktree_status(
            &self,
            _server_url: &ServerUrl,
            request: WorktreeStatusRequest,
        ) -> Result<(u16, WorktreeStatusResponse), WorktreeClientError> {
            self.requests
                .borrow_mut()
                .push((request.method(), request.path(), ""));
            self.status
        }

        fn submit_worktree_sync_once(
            &self,
            _server_url: &ServerUrl,
            request: WorktreeSyncRequest,
        ) -> Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError> {
            self.requests.borrow_mut().push((
                request.method(),
                request.path(),
                request.body_json(),
            ));
            self.sync
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

    fn status(
        lifecycle: WorktreeHostLifecycle,
        readiness: WorktreeReadiness,
        manual: WorktreeManualAvailability,
    ) -> WorktreeStatusResponse {
        WorktreeStatusResponse::from_safe_parts(WorktreeStatusSafeParts {
            configured_mode: WorktreeConfiguredMode::DryRun,
            host_lifecycle: lifecycle,
            readiness,
            readiness_reason: match lifecycle {
                WorktreeHostLifecycle::Disabled => WorktreeReadinessReason::DisabledInert,
                WorktreeHostLifecycle::Starting => WorktreeReadinessReason::Starting,
                WorktreeHostLifecycle::Running => WorktreeReadinessReason::Running,
                WorktreeHostLifecycle::Cancelling => WorktreeReadinessReason::Cancelling,
                WorktreeHostLifecycle::Shutdown => WorktreeReadinessReason::Shutdown,
                WorktreeHostLifecycle::Failed => WorktreeReadinessReason::Failed,
            },
            cycles_completed: u64::MAX,
            cycles_failed: u64::MAX,
            cycle_in_progress: manual == WorktreeManualAvailability::Busy,
            pending_watcher_hints: u64::MAX,
            manual_availability: manual,
        })
    }

    fn fake(
        status: Result<(u16, WorktreeStatusResponse), WorktreeClientError>,
        sync: Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError>,
    ) -> FakeClient {
        FakeClient {
            status,
            sync,
            requests: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn exact_requests_and_empty_body_are_used() {
        let client = fake(
            Ok((
                200,
                status(
                    WorktreeHostLifecycle::Disabled,
                    WorktreeReadiness::Ready,
                    WorktreeManualAvailability::Unavailable,
                ),
            )),
            Ok((
                202,
                WorktreeSyncOnceResponse::submitted(WorktreeSyncOnceSubmissionStatus::Accepted),
            )),
        );
        let _ = render_worktree_status(&configured(), &client);
        let _ = render_worktree_sync_once(&configured(), &client);
        assert_eq!(
            *client.requests.borrow(),
            vec![
                ("GET", WORKTREE_STATUS_PATH, ""),
                ("POST", WORKTREE_SYNC_ONCE_PATH, "{}"),
            ]
        );
    }

    #[test]
    fn running_busy_remains_ready_and_extreme_counters_render() {
        let client = fake(
            Ok((
                200,
                status(
                    WorktreeHostLifecycle::Running,
                    WorktreeReadiness::Ready,
                    WorktreeManualAvailability::Busy,
                ),
            )),
            Err(WorktreeClientError::ServerUnavailable),
        );
        let output = render_worktree_status(&configured(), &client);
        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("worktree readiness: ready"));
        assert!(output.stdout.contains("manual availability: busy"));
        assert!(output.stdout.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn every_sync_outcome_has_exact_exit_classification() {
        let cases = [
            (
                202,
                WorktreeSyncOnceSubmissionStatus::Accepted,
                CliExitCode::Success,
            ),
            (
                409,
                WorktreeSyncOnceSubmissionStatus::Busy,
                CliExitCode::RuntimeError,
            ),
            (
                503,
                WorktreeSyncOnceSubmissionStatus::NotStarted,
                CliExitCode::RuntimeError,
            ),
            (
                503,
                WorktreeSyncOnceSubmissionStatus::Cancelling,
                CliExitCode::RuntimeError,
            ),
            (
                503,
                WorktreeSyncOnceSubmissionStatus::Shutdown,
                CliExitCode::RuntimeError,
            ),
            (
                503,
                WorktreeSyncOnceSubmissionStatus::Unavailable,
                CliExitCode::RuntimeError,
            ),
            (
                500,
                WorktreeSyncOnceSubmissionStatus::Failed,
                CliExitCode::RuntimeError,
            ),
        ];
        for (http, body, expected) in cases {
            let output = classify_sync_response(http, WorktreeSyncOnceResponse::submitted(body));
            assert_eq!(output.exit_code, expected);
            if body == WorktreeSyncOnceSubmissionStatus::Accepted {
                assert!(output.stdout.contains("submitted or queued"));
                assert!(output.stdout.contains("not awaited"));
            }
        }
    }

    #[test]
    fn mismatched_http_and_body_are_rejected() {
        let output = classify_sync_response(
            202,
            WorktreeSyncOnceResponse::submitted(WorktreeSyncOnceSubmissionStatus::Busy),
        );
        assert_eq!(output.exit_code, CliExitCode::RuntimeError);
        assert!(output.stderr.contains("invalid response"));
    }

    #[test]
    fn configuration_and_auth_errors_are_safe() {
        let client = fake(
            Err(WorktreeClientError::Unauthorized),
            Err(WorktreeClientError::Forbidden),
        );
        let missing = render_worktree_status(&CliConfig::default(), &client);
        let unauthorized = render_worktree_status(&configured(), &client);
        let forbidden = render_worktree_sync_once(&configured(), &client);
        assert!(missing.stderr.contains("not configured"));
        assert!(unauthorized.stderr.contains("auth error"));
        assert!(forbidden.stderr.contains("forbidden"));
        for output in [missing, unauthorized, forbidden] {
            for marker in [
                "bearer",
                "route_test_token",
                "token_hash",
                "postgres://",
                "/srv/",
                "ticket",
                "generation",
            ] {
                assert!(!output.stdout.to_lowercase().contains(marker));
                assert!(!output.stderr.to_lowercase().contains(marker));
            }
        }
    }
}
