#![allow(dead_code)]
//! Dependency-free CLI abstraction for read-only Server/API status surfaces.
//!
//! The accepted server/API contracts expose read-only admin status and adapter
//! list endpoints. This module models those request paths, response summaries,
//! safe public error mapping, and human rendering without adding config IO,
//! token loading, provider calls, direct database access, or a concrete HTTP
//! transport dependency.

use crate::{
    config::{CliConfig, ServerUrl},
    output::CliOutput,
};

/// Read-only command execution mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReadCommandMode {
    /// Use configured server settings when present; otherwise report not configured.
    #[default]
    Auto,
    /// Never attempt live server access.
    Offline,
}

/// Accepted read-only Server/API endpoint used by CLI-P4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadEndpoint {
    /// `GET /v1/admin/status`.
    AdminStatus,
    /// `GET /v1/admin/adapters`.
    AdminAdapters,
}

impl ReadEndpoint {
    /// HTTP method for the accepted endpoint.
    #[must_use]
    pub const fn method(self) -> &'static str {
        "GET"
    }

    /// Path for the accepted endpoint.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::AdminStatus => "/v1/admin/status",
            Self::AdminAdapters => "/v1/admin/adapters",
        }
    }
}

/// Request shape a concrete future HTTP transport must execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ServerReadRequest {
    pub endpoint: ReadEndpoint,
}

impl ServerReadRequest {
    /// Build a status request for the accepted admin status endpoint.
    #[must_use]
    pub const fn status() -> Self {
        Self {
            endpoint: ReadEndpoint::AdminStatus,
        }
    }

    /// Build an adapters-list request for the accepted admin adapters endpoint.
    #[must_use]
    pub const fn adapters() -> Self {
        Self {
            endpoint: ReadEndpoint::AdminAdapters,
        }
    }

    /// HTTP method for this request.
    #[must_use]
    pub const fn method(self) -> &'static str {
        self.endpoint.method()
    }

    /// Path for this request.
    #[must_use]
    pub const fn path(self) -> &'static str {
        self.endpoint.path()
    }
}

/// Minimal read-only Server/API client boundary.
pub trait ServerReadClient {
    /// Fetch the accepted status endpoint from an already configured server URL.
    fn fetch_status(&self, server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError>;

    /// Fetch the accepted adapters endpoint from an already configured server URL.
    fn fetch_adapters(&self, server_url: &ServerUrl) -> Result<AdapterList, ServerReadError>;
}

/// Placeholder transport used by the binary until config/token/HTTP wiring is scoped.
#[derive(Clone, Copy, Debug, Default)]
pub struct DeferredHttpClient;

impl ServerReadClient for DeferredHttpClient {
    fn fetch_status(&self, _server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
        Err(ServerReadError::ServerUnavailable)
    }

    fn fetch_adapters(&self, _server_url: &ServerUrl) -> Result<AdapterList, ServerReadError> {
        Err(ServerReadError::ServerUnavailable)
    }
}

/// Safe public runtime error categories for read-only server calls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerReadError {
    /// No server URL is configured.
    NotConfigured,
    /// Operator selected offline mode.
    Offline,
    /// Admin authentication was missing or invalid.
    Unauthorized,
    /// Authenticated principal is not allowed to read admin status.
    Forbidden,
    /// Server could not be reached or returned an unavailable/internal response.
    ServerUnavailable,
    /// Server responded, but reported not-ready state as a failure.
    NotReady,
    /// Response shape was not accepted by the CLI contract.
    InvalidResponse,
}

impl ServerReadError {
    /// Map an API public error code to a CLI-safe category.
    #[must_use]
    pub fn from_public_code(code: &str) -> Self {
        match code {
            "missing_token" | "invalid_token" => Self::Unauthorized,
            "forbidden_role" | "forbidden" => Self::Forbidden,
            "not_ready" => Self::NotReady,
            "service_unavailable" | "internal_error" => Self::ServerUnavailable,
            _ => Self::InvalidResponse,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotConfigured => {
                "not configured: set a server URL before running live read-only commands"
            }
            Self::Offline => "offline: live server calls were not attempted",
            Self::Unauthorized => {
                "auth error: admin token is missing or invalid; no token value was printed"
            }
            Self::Forbidden => "forbidden: configured token cannot read admin status",
            Self::ServerUnavailable => "server unavailable: live status could not be read safely",
            Self::NotReady => "not ready: server is reachable but dependencies are not ready",
            Self::InvalidResponse => "internal error: server response was not accepted by CLI",
        }
    }

    fn into_output(self) -> CliOutput {
        match self {
            Self::NotConfigured | Self::Offline => CliOutput::success(self.message()),
            Self::Unauthorized
            | Self::Forbidden
            | Self::ServerUnavailable
            | Self::NotReady
            | Self::InvalidResponse => CliOutput::runtime_error(self.message()),
        }
    }
}

/// Safe high-level server status values for CLI output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerStatus {
    NotReady,
    Ready,
    Degraded,
    Maintenance,
}

impl ServerStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotReady => "not_ready",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Maintenance => "maintenance",
        }
    }
}

/// Safe dependency readiness state for CLI output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyReadinessState {
    Unknown,
    Ready,
    NotReady,
}

impl DependencyReadinessState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ready => "ready",
            Self::NotReady => "not_ready",
        }
    }
}

/// Safe global pause summary for CLI output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PauseStatusSummary {
    pub supported: bool,
    pub active: Option<bool>,
}

impl PauseStatusSummary {
    /// Unsupported pause status placeholder.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            supported: false,
            active: None,
        }
    }

    fn render(self) -> &'static str {
        match (self.supported, self.active) {
            (false, _) => "unsupported",
            (true, Some(true)) => "active",
            (true, Some(false)) => "inactive",
            (true, None) => "unknown",
        }
    }
}

/// Accepted public status summary used by the CLI renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSummary {
    pub server_status: ServerStatus,
    pub db_readiness_state: DependencyReadinessState,
    pub object_store_readiness_state: DependencyReadinessState,
    pub last_operation_sequence: Option<i64>,
    pub adapter_count: Option<u64>,
    pub pause: PauseStatusSummary,
}

impl StatusSummary {
    /// Dependency-free placeholder matching the accepted API status shape.
    #[must_use]
    pub const fn placeholder() -> Self {
        Self {
            server_status: ServerStatus::NotReady,
            db_readiness_state: DependencyReadinessState::Unknown,
            object_store_readiness_state: DependencyReadinessState::Unknown,
            last_operation_sequence: None,
            adapter_count: None,
            pause: PauseStatusSummary::unsupported(),
        }
    }
}

/// Sanitized adapter cursor summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterCursorSummary {
    pub last_core_seq: Option<i64>,
    pub last_success_at: Option<String>,
    pub has_external_cursor: bool,
}

/// Sanitized adapter summary for CLI output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterSummary {
    pub adapter_id: String,
    pub role: Option<String>,
    pub mode: Option<String>,
    pub enabled: bool,
    pub last_seen_at: Option<String>,
    pub cursor: Option<AdapterCursorSummary>,
}

impl AdapterSummary {
    /// Build a safe adapter summary from public metadata only.
    #[must_use]
    pub fn new(adapter_id: impl Into<String>, enabled: bool) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            role: None,
            mode: None,
            enabled,
            last_seen_at: None,
            cursor: None,
        }
    }
}

/// Accepted public adapters-list summary used by the CLI renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterList {
    pub total_count: u64,
    pub adapters: Vec<AdapterSummary>,
}

impl AdapterList {
    /// Build a list summary with a deterministic total count.
    #[must_use]
    pub fn new(adapters: Vec<AdapterSummary>) -> Self {
        Self {
            total_count: adapters.len() as u64,
            adapters,
        }
    }

    /// Empty placeholder matching the accepted API adapters shape.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

/// Render `haze-sync status` using config and a read-only Server/API client.
#[must_use]
pub fn render_status_command<C>(config: &CliConfig, mode: ReadCommandMode, client: &C) -> CliOutput
where
    C: ServerReadClient,
{
    if mode == ReadCommandMode::Offline {
        return CliOutput::success("status: offline\nlive server calls: not attempted");
    }

    let Some(server_url) = config.server_url.as_ref() else {
        return CliOutput::success(
            "status: not_configured\nserver_url: unset\nlive server calls: not attempted",
        );
    };

    match client.fetch_status(server_url) {
        Ok(summary) => CliOutput::success(render_status_summary(&summary)),
        Err(error) => error.into_output(),
    }
}

/// Render `haze-sync adapters list` using config and a read-only Server/API client.
#[must_use]
pub fn render_adapters_command<C>(
    config: &CliConfig,
    mode: ReadCommandMode,
    client: &C,
) -> CliOutput
where
    C: ServerReadClient,
{
    if mode == ReadCommandMode::Offline {
        return CliOutput::success("adapters: offline\nlive server calls: not attempted");
    }

    let Some(server_url) = config.server_url.as_ref() else {
        return CliOutput::success(
            "adapters: not_configured\nserver_url: unset\nlive server calls: not attempted",
        );
    };

    match client.fetch_adapters(server_url) {
        Ok(adapters) => CliOutput::success(render_adapter_list(&adapters)),
        Err(error) => error.into_output(),
    }
}

fn render_status_summary(summary: &StatusSummary) -> String {
    format!(
        "server status: {}\ndatabase: {}\nobject_store: {}\nlast_operation_sequence: {}\nadapters: {}\npause: {}",
        summary.server_status.as_str(),
        summary.db_readiness_state.as_str(),
        summary.object_store_readiness_state.as_str(),
        render_optional_i64(summary.last_operation_sequence),
        render_optional_u64(summary.adapter_count),
        summary.pause.render()
    )
}

fn render_adapter_list(list: &AdapterList) -> String {
    if list.adapters.is_empty() {
        return format!("adapters: {} configured", list.total_count);
    }

    let mut lines = vec![format!("adapters: {} configured", list.total_count)];
    for adapter in &list.adapters {
        lines.push(render_adapter_summary(adapter));
    }
    lines.join("\n")
}

fn render_adapter_summary(adapter: &AdapterSummary) -> String {
    format!(
        "- {} role={} mode={} enabled={} last_seen={} cursor={}",
        adapter.adapter_id,
        adapter.role.as_deref().unwrap_or("unknown"),
        adapter.mode.as_deref().unwrap_or("unknown"),
        adapter.enabled,
        adapter.last_seen_at.as_deref().unwrap_or("unknown"),
        render_cursor(adapter.cursor.as_ref())
    )
}

fn render_cursor(cursor: Option<&AdapterCursorSummary>) -> &'static str {
    match cursor {
        None => "unknown",
        Some(cursor) if cursor.has_external_cursor => "present",
        Some(_) => "absent",
    }
}

fn render_optional_i64(value: Option<i64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn render_optional_u64(value: Option<u64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{OutputFormat, ProfileName, TokenSource};
    use crate::output::CliExitCode;

    #[derive(Clone, Debug)]
    struct FakeReadClient {
        status: Result<StatusSummary, ServerReadError>,
        adapters: Result<AdapterList, ServerReadError>,
    }

    impl ServerReadClient for FakeReadClient {
        fn fetch_status(&self, _server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
            self.status.clone()
        }

        fn fetch_adapters(&self, _server_url: &ServerUrl) -> Result<AdapterList, ServerReadError> {
            self.adapters.clone()
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
                "output leaked forbidden marker {marker}: {output}"
            );
        }
    }

    #[test]
    fn accepted_admin_requests_use_get_and_contract_paths() {
        let status = ServerReadRequest::status();
        let adapters = ServerReadRequest::adapters();

        assert_eq!(status.method(), "GET");
        assert_eq!(status.path(), "/v1/admin/status");
        assert_eq!(adapters.method(), "GET");
        assert_eq!(adapters.path(), "/v1/admin/adapters");
    }

    #[test]
    fn status_without_server_config_reports_not_configured_without_live_call() {
        let client = FakeReadClient {
            status: Ok(StatusSummary::placeholder()),
            adapters: Ok(AdapterList::empty()),
        };
        let output = render_status_command(&CliConfig::default(), ReadCommandMode::Auto, &client);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("status: not_configured"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn adapters_without_server_config_reports_not_configured_without_live_call() {
        let client = FakeReadClient {
            status: Ok(StatusSummary::placeholder()),
            adapters: Ok(AdapterList::empty()),
        };
        let output = render_adapters_command(&CliConfig::default(), ReadCommandMode::Auto, &client);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("adapters: not_configured"));
        assert!(output.stdout.contains("live server calls: not attempted"));
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn offline_mode_never_attempts_live_calls() {
        let client = FakeReadClient {
            status: Err(ServerReadError::ServerUnavailable),
            adapters: Err(ServerReadError::ServerUnavailable),
        };

        let status = render_status_command(&configured(), ReadCommandMode::Offline, &client);
        let adapters = render_adapters_command(&configured(), ReadCommandMode::Offline, &client);

        assert_eq!(status.exit_code, CliExitCode::Success);
        assert_eq!(adapters.exit_code, CliExitCode::Success);
        assert!(status.stdout.contains("status: offline"));
        assert!(adapters.stdout.contains("adapters: offline"));
    }

    #[test]
    fn live_status_summary_renders_safe_human_output() {
        let client = FakeReadClient {
            status: Ok(StatusSummary {
                server_status: ServerStatus::Ready,
                db_readiness_state: DependencyReadinessState::Ready,
                object_store_readiness_state: DependencyReadinessState::Ready,
                last_operation_sequence: Some(42),
                adapter_count: Some(1),
                pause: PauseStatusSummary {
                    supported: true,
                    active: Some(false),
                },
            }),
            adapters: Ok(AdapterList::empty()),
        };
        let output = render_status_command(&configured(), ReadCommandMode::Auto, &client);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("server status: ready"));
        assert!(output.stdout.contains("last_operation_sequence: 42"));
        assert!(output.stdout.contains("pause: inactive"));
        assert_no_sensitive_markers(&output.stdout);
    }

    #[test]
    fn live_adapter_list_renders_safe_human_output_without_raw_cursor() {
        let mut adapter = AdapterSummary::new("gdrive-adapter", true);
        adapter.role = Some("gdrive_adapter".to_owned());
        adapter.mode = Some("import_only".to_owned());
        adapter.last_seen_at = Some("2026-07-02T10:00:00Z".to_owned());
        adapter.cursor = Some(AdapterCursorSummary {
            last_core_seq: Some(42),
            last_success_at: Some("2026-07-02T10:01:00Z".to_owned()),
            has_external_cursor: true,
        });
        let client = FakeReadClient {
            status: Ok(StatusSummary::placeholder()),
            adapters: Ok(AdapterList::new(vec![adapter])),
        };
        let output = render_adapters_command(&configured(), ReadCommandMode::Auto, &client);

        assert_eq!(output.exit_code, CliExitCode::Success);
        assert!(output.stdout.contains("adapters: 1 configured"));
        assert!(output.stdout.contains("gdrive-adapter"));
        assert!(output.stdout.contains("cursor=present"));
        assert!(!output.stdout.contains("external_cursor_json"));
        assert_no_sensitive_markers(&output.stdout);
    }

    #[test]
    fn public_api_error_codes_map_to_safe_cli_categories() {
        assert_eq!(
            ServerReadError::from_public_code("missing_token"),
            ServerReadError::Unauthorized
        );
        assert_eq!(
            ServerReadError::from_public_code("forbidden_role"),
            ServerReadError::Forbidden
        );
        assert_eq!(
            ServerReadError::from_public_code("internal_error"),
            ServerReadError::ServerUnavailable
        );
        assert_eq!(
            ServerReadError::from_public_code("unexpected_new_code"),
            ServerReadError::InvalidResponse
        );
    }

    #[test]
    fn live_errors_render_to_stderr_without_secrets() {
        let client = FakeReadClient {
            status: Err(ServerReadError::Unauthorized),
            adapters: Err(ServerReadError::Forbidden),
        };
        let status = render_status_command(&configured(), ReadCommandMode::Auto, &client);
        let adapters = render_adapters_command(&configured(), ReadCommandMode::Auto, &client);

        assert_eq!(status.exit_code, CliExitCode::RuntimeError);
        assert_eq!(adapters.exit_code, CliExitCode::RuntimeError);
        assert!(status.stdout.is_empty());
        assert!(adapters.stdout.is_empty());
        assert!(status.stderr.contains("auth error"));
        assert!(adapters.stderr.contains("forbidden"));
        assert_no_sensitive_markers(&status.stderr);
        assert_no_sensitive_markers(&adapters.stderr);
    }
}
