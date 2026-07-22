//! Bounded HTTP client for the Server-owned Google Drive durable-state contract.
//!
//! This module owns transport, strict decoding, mode-aware call gating, and safe
//! retry classification only. It does not access Storage, call Google, reconcile
//! state, schedule work, or retry compare-and-commit requests automatically.

use crate::config::{AdapterMode, SecretString};
use crate::identity::AdapterIdentity;
use haze_sync_api::contracts::headers::{
    BearerToken, IdempotencyKey, AUTHORIZATION_HEADER, IDEMPOTENCY_KEY_HEADER,
};
use haze_sync_api::dto::gdrive::{
    GDriveLastOperationsSummaryDto, GDriveMappingFactsDto, GDriveStateCommitRequest,
    GDriveStateCommitResponse, GDriveStateErrorCode, GDriveStateErrorResponse,
    GDriveStateSnapshotResponse,
};
use haze_sync_api::routes::gdrive::{
    validate_private_snapshot, GDRIVE_STATE_COMMIT_ROUTE, GDRIVE_STATE_ROUTE,
    MAX_GDRIVE_STATE_LIMIT,
};
use haze_sync_common::VaultPath;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};
use std::time::Duration;

const CONTENT_TYPE_HEADER: &str = "Content-Type";
const JSON_CONTENT_TYPE: &str = "application/json";
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_PAGES: usize = 100;
const DEFAULT_MAX_ITEMS: usize = 10_000;
const MAX_SAFE_ERROR_MESSAGE_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
}

/// Private HTTP request surface passed to an adapter-owned transport.
///
/// Debug and Display intentionally omit the URL, identity, token, query values,
/// idempotency key, and body.
#[derive(Clone)]
pub struct HttpRequest {
    method: HttpMethod,
    url: String,
    bearer_token: BearerToken,
    idempotency_key: Option<IdempotencyKey>,
    content_type: Option<&'static str>,
    body: Vec<u8>,
}

impl HttpRequest {
    #[must_use]
    pub const fn method(&self) -> HttpMethod {
        self.method
    }

    #[must_use]
    pub fn expose_url_for_transport(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn authorization_header_for_transport(&self) -> String {
        format!("Bearer {}", self.bearer_token.expose_for_auth())
    }

    #[must_use]
    pub fn idempotency_key_for_transport(&self) -> Option<&str> {
        self.idempotency_key.as_ref().map(IdempotencyKey::as_str)
    }

    #[must_use]
    pub const fn content_type_for_transport(&self) -> Option<&'static str> {
        self.content_type
    }

    #[must_use]
    pub fn body_for_transport(&self) -> &[u8] {
        &self.body
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &"<redacted-private-url>")
            .field("authorization", &"<redacted-bearer-token>")
            .field("idempotency_key", &"<redacted-idempotency-key>")
            .field("content_type", &self.content_type)
            .field("body", &"<redacted-private-json>")
            .finish()
    }
}

impl fmt::Display for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Google Drive durable-state HTTP request (<redacted>)")
    }
}

/// Bounded response surface returned by a transport.
#[derive(Clone, Eq, PartialEq)]
pub struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

impl HttpResponse {
    #[must_use]
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn body_for_decoder(&self) -> &[u8] {
        &self.body
    }
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("body", &"<redacted-response-body>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpTransportError {
    Timeout,
    Unavailable,
}

impl fmt::Display for HttpTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Timeout => "HTTP transport timed out",
            Self::Unavailable => "HTTP transport is unavailable",
        })
    }
}

impl Error for HttpTransportError {}

/// Fakeable single-request transport boundary.
pub trait HttpTransport {
    fn execute(
        &self,
        request: &HttpRequest,
        max_response_bytes: usize,
    ) -> Result<HttpResponse, HttpTransportError>;
}

/// Concrete synchronous HTTP transport with redirects disabled and bounded IO.
pub struct UreqHttpTransport {
    agent: ureq::Agent,
}

impl UreqHttpTransport {
    #[must_use]
    pub fn new(policy: HttpClientPolicy) -> Self {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(policy.connect_timeout)
            .timeout(policy.request_timeout)
            .timeout_read(policy.request_timeout)
            .timeout_write(policy.request_timeout)
            .build();
        Self { agent }
    }
}

impl fmt::Debug for UreqHttpTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "UreqHttpTransport(redirects=disabled, bounded_connect=true, overall_deadline=true)",
        )
    }
}

impl HttpTransport for UreqHttpTransport {
    fn execute(
        &self,
        request: &HttpRequest,
        max_response_bytes: usize,
    ) -> Result<HttpResponse, HttpTransportError> {
        let authorization = request.authorization_header_for_transport();
        let mut outbound = match request.method() {
            HttpMethod::Get => self.agent.get(request.expose_url_for_transport()),
            HttpMethod::Post => self.agent.post(request.expose_url_for_transport()),
        }
        .set(AUTHORIZATION_HEADER, &authorization);

        if let Some(content_type) = request.content_type_for_transport() {
            outbound = outbound.set(CONTENT_TYPE_HEADER, content_type);
        }
        if let Some(idempotency_key) = request.idempotency_key_for_transport() {
            outbound = outbound.set(IDEMPOTENCY_KEY_HEADER, idempotency_key);
        }

        let result = match request.method() {
            HttpMethod::Get => outbound.call(),
            HttpMethod::Post => outbound.send_bytes(request.body_for_transport()),
        };
        let response = match result {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(error)) => {
                return Err(classify_ureq_transport_error(&error));
            }
        };
        read_bounded_ureq_response(response, max_response_bytes)
    }
}

fn classify_ureq_transport_error(error: &ureq::Transport) -> HttpTransportError {
    let mut cause: Option<&(dyn Error + 'static)> = Some(error);
    while let Some(current) = cause {
        if let Some(io_error) = current.downcast_ref::<io::Error>() {
            if matches!(
                io_error.kind(),
                io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
            ) {
                return HttpTransportError::Timeout;
            }
        }
        cause = current.source();
    }

    if error.kind() == ureq::ErrorKind::Io
        && error
            .message()
            .is_some_and(|message| message.to_ascii_lowercase().contains("timed out"))
    {
        HttpTransportError::Timeout
    } else {
        HttpTransportError::Unavailable
    }
}

fn read_bounded_ureq_response(
    response: ureq::Response,
    max_response_bytes: usize,
) -> Result<HttpResponse, HttpTransportError> {
    let status = response.status();
    let read_limit = max_response_bytes.saturating_add(1) as u64;
    let mut body = Vec::new();
    response
        .into_reader()
        .take(read_limit)
        .read_to_end(&mut body)
        .map_err(map_io_error)?;
    Ok(HttpResponse::new(status, body))
}

fn map_io_error(error: io::Error) -> HttpTransportError {
    if matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ) {
        HttpTransportError::Timeout
    } else {
        HttpTransportError::Unavailable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpClientPolicy {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_response_bytes: usize,
    pub max_pages: usize,
    pub max_items: usize,
}

impl HttpClientPolicy {
    pub fn new(
        connect_timeout: Duration,
        request_timeout: Duration,
        max_response_bytes: usize,
        max_pages: usize,
        max_items: usize,
    ) -> Result<Self, DurableStateClientError> {
        if connect_timeout.is_zero()
            || request_timeout.is_zero()
            || max_response_bytes == 0
            || max_pages == 0
            || max_items == 0
        {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::InvalidRequest,
            ));
        }
        Ok(Self {
            connect_timeout,
            request_timeout,
            max_response_bytes,
            max_pages,
            max_items,
        })
    }
}

impl Default for HttpClientPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            max_pages: DEFAULT_MAX_PAGES,
            max_items: DEFAULT_MAX_ITEMS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableStateErrorCategory {
    ModeDenied,
    InvalidRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    StateVersionMismatch,
    InvalidCursorState,
    StaleState,
    CursorRegression,
    CursorGap,
    MappingConflict,
    IdempotencyConflict,
    Validation,
    TransportTimeout,
    TransportUnavailable,
    ServiceUnavailable,
    Internal,
    MalformedResponse,
    ResponseTooLarge,
    RedirectRefused,
    PaginationLoop,
    PaginationLimit,
    CollectionTooLarge,
    InconsistentSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DurableStateClientError {
    category: DurableStateErrorCategory,
    retryable: bool,
    message: &'static str,
}

impl DurableStateClientError {
    const fn new(category: DurableStateErrorCategory) -> Self {
        let retryable = matches!(
            category,
            DurableStateErrorCategory::TransportTimeout
                | DurableStateErrorCategory::TransportUnavailable
                | DurableStateErrorCategory::ServiceUnavailable
                | DurableStateErrorCategory::Internal
        );
        let message = match category {
            DurableStateErrorCategory::ModeDenied => {
                "adapter mode does not permit this durable-state call"
            }
            DurableStateErrorCategory::InvalidRequest => "durable-state request is invalid",
            DurableStateErrorCategory::Unauthorized => "durable-state authentication failed",
            DurableStateErrorCategory::Forbidden => "durable-state access is forbidden",
            DurableStateErrorCategory::NotFound => "durable state was not found",
            DurableStateErrorCategory::StateVersionMismatch => {
                "durable-state version does not match"
            }
            DurableStateErrorCategory::InvalidCursorState => "durable cursor state is invalid",
            DurableStateErrorCategory::StaleState => "durable state is stale",
            DurableStateErrorCategory::CursorRegression => "durable cursor would regress",
            DurableStateErrorCategory::CursorGap => "durable cursor contains a gap",
            DurableStateErrorCategory::MappingConflict => "durable mapping facts conflict",
            DurableStateErrorCategory::IdempotencyConflict => {
                "durable-state idempotency metadata conflicts"
            }
            DurableStateErrorCategory::Validation => "durable-state validation failed",
            DurableStateErrorCategory::TransportTimeout => "durable-state transport timed out",
            DurableStateErrorCategory::TransportUnavailable => {
                "durable-state transport is unavailable"
            }
            DurableStateErrorCategory::ServiceUnavailable => "durable-state service is unavailable",
            DurableStateErrorCategory::Internal => "durable-state service failed safely",
            DurableStateErrorCategory::MalformedResponse => "durable-state response is malformed",
            DurableStateErrorCategory::ResponseTooLarge => {
                "durable-state response exceeds the configured bound"
            }
            DurableStateErrorCategory::RedirectRefused => "durable-state redirect was refused",
            DurableStateErrorCategory::PaginationLoop => "durable-state pagination did not advance",
            DurableStateErrorCategory::PaginationLimit => {
                "durable-state pagination exceeded the configured bound"
            }
            DurableStateErrorCategory::CollectionTooLarge => {
                "durable-state collection exceeds the configured bound"
            }
            DurableStateErrorCategory::InconsistentSnapshot => {
                "durable-state pages do not describe one stable snapshot"
            }
        };
        Self {
            category,
            retryable,
            message,
        }
    }

    #[must_use]
    pub const fn category(&self) -> DurableStateErrorCategory {
        self.category
    }

    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for DurableStateClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for DurableStateClientError {}

impl From<HttpTransportError> for DurableStateClientError {
    fn from(error: HttpTransportError) -> Self {
        match error {
            HttpTransportError::Timeout => Self::new(DurableStateErrorCategory::TransportTimeout),
            HttpTransportError::Unavailable => {
                Self::new(DurableStateErrorCategory::TransportUnavailable)
            }
        }
    }
}

/// Stable, bounded collection assembled from private snapshot pages.
pub struct CollectedGDriveState {
    state_format_version: u32,
    state_version: u64,
    cursor_generation: u64,
    core_export_checkpoint: u64,
    last_operations: GDriveLastOperationsSummaryDto,
    mappings: Vec<GDriveMappingFactsDto>,
    page_count: usize,
}

impl CollectedGDriveState {
    #[must_use]
    pub const fn state_format_version(&self) -> u32 {
        self.state_format_version
    }

    #[must_use]
    pub const fn state_version(&self) -> u64 {
        self.state_version
    }

    #[must_use]
    pub const fn cursor_generation(&self) -> u64 {
        self.cursor_generation
    }

    #[must_use]
    pub const fn core_export_checkpoint(&self) -> u64 {
        self.core_export_checkpoint
    }

    #[must_use]
    pub const fn last_operations(&self) -> &GDriveLastOperationsSummaryDto {
        &self.last_operations
    }

    #[must_use]
    pub fn mappings(&self) -> &[GDriveMappingFactsDto] {
        &self.mappings
    }

    #[must_use]
    pub const fn page_count(&self) -> usize {
        self.page_count
    }
}

impl fmt::Debug for CollectedGDriveState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollectedGDriveState")
            .field("state_format_version", &self.state_format_version)
            .field("state_version", &self.state_version)
            .field("cursor_generation", &self.cursor_generation)
            .field("core_export_checkpoint", &self.core_export_checkpoint)
            .field("last_operations", &"<redacted-operation-metadata>")
            .field("mappings", &"<redacted-private-mappings>")
            .field("page_count", &self.page_count)
            .finish()
    }
}

pub trait DurableStateClient {
    fn get_state_page(
        &self,
        after_path: Option<&VaultPath>,
        limit: usize,
    ) -> Result<GDriveStateSnapshotResponse, DurableStateClientError>;

    fn collect_state(&self, limit: usize) -> Result<CollectedGDriveState, DurableStateClientError>;

    fn compare_and_commit(
        &self,
        idempotency_key: &IdempotencyKey,
        request: &GDriveStateCommitRequest,
    ) -> Result<GDriveStateCommitResponse, DurableStateClientError>;
}

/// Concrete API-contract client over a fakeable HTTP transport.
pub struct HttpDurableStateClient<T> {
    base_url: String,
    identity: AdapterIdentity,
    bearer_token: BearerToken,
    transport: T,
    policy: HttpClientPolicy,
}

impl<T> HttpDurableStateClient<T> {
    pub fn new(
        server_url: &str,
        identity: AdapterIdentity,
        adapter_token: &SecretString,
        transport: T,
        policy: HttpClientPolicy,
    ) -> Result<Self, DurableStateClientError> {
        let base_url = validate_and_normalize_server_url(server_url)?;
        let bearer_token = BearerToken::new(adapter_token.expose_secret().to_owned())
            .map_err(|_| DurableStateClientError::new(DurableStateErrorCategory::InvalidRequest))?;
        Ok(Self {
            base_url,
            identity,
            bearer_token,
            transport,
            policy,
        })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    fn state_request(
        &self,
        after_path: Option<&VaultPath>,
        limit: usize,
    ) -> Result<HttpRequest, DurableStateClientError> {
        if limit == 0 || limit > MAX_GDRIVE_STATE_LIMIT {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::InvalidRequest,
            ));
        }
        let route = route_for_identity(GDRIVE_STATE_ROUTE, &self.identity);
        let mut url = format!("{}{}?", self.base_url, route);
        if let Some(after_path) = after_path {
            url.push_str("after_path=");
            url.push_str(&percent_encode_query(after_path.as_str()));
            url.push('&');
        }
        url.push_str("limit=");
        url.push_str(&limit.to_string());
        Ok(HttpRequest {
            method: HttpMethod::Get,
            url,
            bearer_token: self.bearer_token.clone(),
            idempotency_key: None,
            content_type: None,
            body: Vec::new(),
        })
    }

    fn commit_request(
        &self,
        idempotency_key: &IdempotencyKey,
        body: &GDriveStateCommitRequest,
    ) -> Result<HttpRequest, DurableStateClientError> {
        let route = route_for_identity(GDRIVE_STATE_COMMIT_ROUTE, &self.identity);
        let body = serde_json::to_vec(body)
            .map_err(|_| DurableStateClientError::new(DurableStateErrorCategory::InvalidRequest))?;
        Ok(HttpRequest {
            method: HttpMethod::Post,
            url: format!("{}{}", self.base_url, route),
            bearer_token: self.bearer_token.clone(),
            idempotency_key: Some(idempotency_key.clone()),
            content_type: Some(JSON_CONTENT_TYPE),
            body,
        })
    }

    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, DurableStateClientError>
    where
        T: HttpTransport,
    {
        let response = self
            .transport
            .execute(request, self.policy.max_response_bytes)?;
        if response.body_for_decoder().len() > self.policy.max_response_bytes {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::ResponseTooLarge,
            ));
        }
        if (300..400).contains(&response.status()) {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::RedirectRefused,
            ));
        }
        Ok(response)
    }
}

impl<T> fmt::Debug for HttpDurableStateClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpDurableStateClient")
            .field("base_url", &"<redacted-server-endpoint>")
            .field("identity", &self.identity)
            .field("bearer_token", &self.bearer_token)
            .field("transport", &"<transport>")
            .field("policy", &self.policy)
            .finish()
    }
}

impl<T: HttpTransport> DurableStateClient for HttpDurableStateClient<T> {
    fn get_state_page(
        &self,
        after_path: Option<&VaultPath>,
        limit: usize,
    ) -> Result<GDriveStateSnapshotResponse, DurableStateClientError> {
        let request = self.state_request(after_path, limit)?;
        let response = self.execute(&request)?;
        if response.status() != 200 {
            return Err(decode_route_error(&response)?);
        }
        let snapshot: GDriveStateSnapshotResponse =
            serde_json::from_slice(response.body_for_decoder()).map_err(|_| {
                DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
            })?;
        validate_private_snapshot(&snapshot).map_err(|_| {
            DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
        })?;
        if snapshot.adapter_id.as_str() != self.identity.expose_for_route() {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::MalformedResponse,
            ));
        }
        Ok(snapshot)
    }

    fn collect_state(&self, limit: usize) -> Result<CollectedGDriveState, DurableStateClientError> {
        let mut after_path: Option<VaultPath> = None;
        let mut seen_paths = BTreeSet::new();
        let mut baseline: Option<SnapshotSignature> = None;
        let mut mappings = Vec::new();
        let mut page_count = 0_usize;

        loop {
            if page_count >= self.policy.max_pages {
                return Err(DurableStateClientError::new(
                    DurableStateErrorCategory::PaginationLimit,
                ));
            }
            let page = self.get_state_page(after_path.as_ref(), limit)?;
            page_count += 1;
            let signature = SnapshotSignature::from_snapshot(&page);
            if let Some(expected) = baseline.as_ref() {
                if expected != &signature {
                    return Err(DurableStateClientError::new(
                        DurableStateErrorCategory::InconsistentSnapshot,
                    ));
                }
            } else {
                baseline = Some(signature);
            }

            let next_total = mappings
                .len()
                .checked_add(page.mappings.len())
                .ok_or_else(|| {
                    DurableStateClientError::new(DurableStateErrorCategory::CollectionTooLarge)
                })?;
            if next_total > self.policy.max_items {
                return Err(DurableStateClientError::new(
                    DurableStateErrorCategory::CollectionTooLarge,
                ));
            }
            mappings.extend(page.mappings);

            let Some(next_dto) = page.next_after_path else {
                break;
            };
            let next_path = VaultPath::try_from(next_dto).map_err(|_| {
                DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
            })?;
            if after_path
                .as_ref()
                .is_some_and(|current| next_path.as_str() <= current.as_str())
                || !seen_paths.insert(next_path.as_str().to_owned())
            {
                return Err(DurableStateClientError::new(
                    DurableStateErrorCategory::PaginationLoop,
                ));
            }
            after_path = Some(next_path);
        }

        let baseline = baseline.ok_or_else(|| {
            DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
        })?;
        Ok(CollectedGDriveState {
            state_format_version: baseline.state_format_version,
            state_version: baseline.state_version,
            cursor_generation: baseline.cursor_generation,
            core_export_checkpoint: baseline.core_export_checkpoint,
            last_operations: baseline.last_operations,
            mappings,
            page_count,
        })
    }

    fn compare_and_commit(
        &self,
        idempotency_key: &IdempotencyKey,
        request: &GDriveStateCommitRequest,
    ) -> Result<GDriveStateCommitResponse, DurableStateClientError> {
        let outbound = self.commit_request(idempotency_key, request)?;
        let response = self.execute(&outbound)?;
        match response.status() {
            200 => decode_commit_response(200, response.body_for_decoder()),
            409 | 422 => decode_commit_response(response.status(), response.body_for_decoder())
                .or_else(|_| Err(decode_route_error(&response)?)),
            _ => Err(decode_route_error(&response)?),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
struct SnapshotSignature {
    state_format_version: u32,
    state_version: u64,
    cursor_generation: u64,
    cursor_present: bool,
    core_export_checkpoint: u64,
    last_operations: GDriveLastOperationsSummaryDto,
}

impl SnapshotSignature {
    fn from_snapshot(snapshot: &GDriveStateSnapshotResponse) -> Self {
        Self {
            state_format_version: snapshot.state_format_version,
            state_version: snapshot.state_version,
            cursor_generation: snapshot.cursor.generation,
            cursor_present: snapshot.cursor.present,
            core_export_checkpoint: snapshot.core_export_checkpoint,
            last_operations: snapshot.last_operations.clone(),
        }
    }
}

/// Local capability gate around a durable-state client.
pub struct ModeAwareDurableStateClient<C> {
    mode: AdapterMode,
    inner: C,
}

impl<C> ModeAwareDurableStateClient<C> {
    #[must_use]
    pub const fn new(mode: AdapterMode, inner: C) -> Self {
        Self { mode, inner }
    }

    #[must_use]
    pub const fn inner(&self) -> &C {
        &self.inner
    }
}

impl<C> fmt::Debug for ModeAwareDurableStateClient<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModeAwareDurableStateClient")
            .field("mode", &self.mode)
            .field("inner", &"<durable-state-client>")
            .finish()
    }
}

impl<C: DurableStateClient> DurableStateClient for ModeAwareDurableStateClient<C> {
    fn get_state_page(
        &self,
        after_path: Option<&VaultPath>,
        limit: usize,
    ) -> Result<GDriveStateSnapshotResponse, DurableStateClientError> {
        if !self.mode.permits_core_reads() {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::ModeDenied,
            ));
        }
        self.inner.get_state_page(after_path, limit)
    }

    fn collect_state(&self, limit: usize) -> Result<CollectedGDriveState, DurableStateClientError> {
        if !self.mode.permits_core_reads() {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::ModeDenied,
            ));
        }
        self.inner.collect_state(limit)
    }

    fn compare_and_commit(
        &self,
        idempotency_key: &IdempotencyKey,
        request: &GDriveStateCommitRequest,
    ) -> Result<GDriveStateCommitResponse, DurableStateClientError> {
        if !self.mode.permits_durable_state_mutation() {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::ModeDenied,
            ));
        }
        self.inner.compare_and_commit(idempotency_key, request)
    }
}

fn validate_and_normalize_server_url(server_url: &str) -> Result<String, DurableStateClientError> {
    if server_url.is_empty()
        || server_url.trim() != server_url
        || server_url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        || server_url.contains('\\')
        || server_url.contains('?')
        || server_url.contains('#')
    {
        return Err(invalid_request());
    }

    let (scheme, rest) = if let Some(rest) = server_url.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = server_url.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err(invalid_request());
    };

    let authority = if let Some(authority) = rest.strip_suffix('/') {
        if authority.contains('/') {
            return Err(invalid_request());
        }
        authority
    } else {
        if rest.contains('/') {
            return Err(invalid_request());
        }
        rest
    };

    validate_authority(authority)?;
    Ok(format!("{scheme}://{authority}"))
}

fn validate_authority(authority: &str) -> Result<(), DurableStateClientError> {
    if authority.is_empty() || authority.contains('@') {
        return Err(invalid_request());
    }

    if let Some(bracketed) = authority.strip_prefix('[') {
        let close = bracketed.find(']').ok_or_else(invalid_request)?;
        let host = &bracketed[..close];
        let suffix = &bracketed[close + 1..];
        if host.is_empty()
            || !host.contains(':')
            || !host
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || matches!(byte, b':' | b'.'))
        {
            return Err(invalid_request());
        }
        validate_optional_port(suffix)
    } else {
        let mut pieces = authority.split(':');
        let host = pieces.next().unwrap_or_default();
        let port = pieces.next();
        if pieces.next().is_some() || !valid_dns_or_ipv4_host(host) {
            return Err(invalid_request());
        }
        if let Some(port) = port {
            validate_port(port)?;
        }
        Ok(())
    }
}

fn validate_optional_port(suffix: &str) -> Result<(), DurableStateClientError> {
    if suffix.is_empty() {
        Ok(())
    } else if let Some(port) = suffix.strip_prefix(':') {
        validate_port(port)
    } else {
        Err(invalid_request())
    }
}

fn validate_port(port: &str) -> Result<(), DurableStateClientError> {
    if port.is_empty()
        || !port.bytes().all(|byte| byte.is_ascii_digit())
        || port
            .parse::<u16>()
            .ok()
            .filter(|value| *value != 0)
            .is_none()
    {
        Err(invalid_request())
    } else {
        Ok(())
    }
}

fn valid_dns_or_ipv4_host(host: &str) -> bool {
    if host.is_empty()
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
    {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty() && !label.starts_with('-') && !label.ends_with('-') && label.len() <= 63
    })
}

fn invalid_request() -> DurableStateClientError {
    DurableStateClientError::new(DurableStateErrorCategory::InvalidRequest)
}

fn route_for_identity(template: &str, identity: &AdapterIdentity) -> String {
    template.replace("{adapter_id}", identity.expose_for_route())
}

fn percent_encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

const fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'A' + (value - 10)) as char,
    }
}

fn decode_commit_response(
    status: u16,
    body: &[u8],
) -> Result<GDriveStateCommitResponse, DurableStateClientError> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse))?;
    let object = value.as_object().ok_or_else(|| {
        DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
    })?;
    let response_status = object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse)
        })?;
    let expected_keys: &[&str] = match response_status {
        "committed" => &[
            "status",
            "state_version",
            "cursor_generation",
            "core_export_checkpoint",
        ],
        "replayed" => &["status", "state_version"],
        "stale_state"
        | "cursor_regression"
        | "cursor_gap"
        | "mapping_conflict"
        | "idempotency_conflict"
        | "validation_failed" => &["status"],
        _ => {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::MalformedResponse,
            ))
        }
    };
    if object.len() != expected_keys.len()
        || !expected_keys.iter().all(|key| object.contains_key(*key))
    {
        return Err(DurableStateClientError::new(
            DurableStateErrorCategory::MalformedResponse,
        ));
    }
    let response: GDriveStateCommitResponse = serde_json::from_value(value)
        .map_err(|_| DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse))?;
    let status_matches = matches!(
        (&response, status),
        (
            GDriveStateCommitResponse::Committed { .. }
                | GDriveStateCommitResponse::Replayed { .. },
            200,
        ) | (
            GDriveStateCommitResponse::StaleState
                | GDriveStateCommitResponse::CursorRegression
                | GDriveStateCommitResponse::CursorGap
                | GDriveStateCommitResponse::MappingConflict
                | GDriveStateCommitResponse::IdempotencyConflict,
            409,
        ) | (GDriveStateCommitResponse::ValidationFailed, 422)
    );
    if !status_matches {
        return Err(DurableStateClientError::new(
            DurableStateErrorCategory::MalformedResponse,
        ));
    }
    Ok(response)
}

fn decode_route_error(
    response: &HttpResponse,
) -> Result<DurableStateClientError, DurableStateClientError> {
    let envelope: GDriveStateErrorResponse = serde_json::from_slice(response.body_for_decoder())
        .map_err(|_| DurableStateClientError::new(DurableStateErrorCategory::MalformedResponse))?;
    if envelope.error.message.is_empty()
        || envelope.error.message.len() > MAX_SAFE_ERROR_MESSAGE_BYTES
        || envelope.error.message.chars().any(char::is_control)
    {
        return Err(DurableStateClientError::new(
            DurableStateErrorCategory::MalformedResponse,
        ));
    }
    let category = match (response.status(), envelope.error.code) {
        (401, GDriveStateErrorCode::Unauthorized) => DurableStateErrorCategory::Unauthorized,
        (403, GDriveStateErrorCode::Forbidden) => DurableStateErrorCategory::Forbidden,
        (404, GDriveStateErrorCode::AdapterNotFound) => DurableStateErrorCategory::NotFound,
        (409, GDriveStateErrorCode::StateVersionMismatch) => {
            DurableStateErrorCategory::StateVersionMismatch
        }
        (409, GDriveStateErrorCode::InvalidCursorState) => {
            DurableStateErrorCategory::InvalidCursorState
        }
        (409, GDriveStateErrorCode::StaleState) => DurableStateErrorCategory::StaleState,
        (409, GDriveStateErrorCode::CursorRegression) => {
            DurableStateErrorCategory::CursorRegression
        }
        (409, GDriveStateErrorCode::CursorGap) => DurableStateErrorCategory::CursorGap,
        (409, GDriveStateErrorCode::MappingConflict) => DurableStateErrorCategory::MappingConflict,
        (409, GDriveStateErrorCode::IdempotencyConflict) => {
            DurableStateErrorCategory::IdempotencyConflict
        }
        (422, GDriveStateErrorCode::ValidationError) => DurableStateErrorCategory::Validation,
        (500, GDriveStateErrorCode::Internal) => DurableStateErrorCategory::Internal,
        (503, GDriveStateErrorCode::Unavailable) => DurableStateErrorCategory::ServiceUnavailable,
        _ => {
            return Err(DurableStateClientError::new(
                DurableStateErrorCategory::MalformedResponse,
            ))
        }
    };
    Ok(DurableStateClientError::new(category))
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::contracts::headers::IdempotencyKey;
    use haze_sync_api::dto::gdrive::MAX_GDRIVE_STATE_ITEMS;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::Write as _;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Instant;

    #[derive(Default)]
    struct FakeHttpTransport {
        calls: RefCell<Vec<HttpRequest>>,
        responses: RefCell<VecDeque<Result<HttpResponse, HttpTransportError>>>,
    }

    impl FakeHttpTransport {
        fn with_responses(
            responses: impl IntoIterator<Item = Result<HttpResponse, HttpTransportError>>,
        ) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                responses: RefCell::new(responses.into_iter().collect()),
            }
        }

        fn calls(&self) -> std::cell::Ref<'_, Vec<HttpRequest>> {
            self.calls.borrow()
        }
    }

    impl HttpTransport for FakeHttpTransport {
        fn execute(
            &self,
            request: &HttpRequest,
            _max_response_bytes: usize,
        ) -> Result<HttpResponse, HttpTransportError> {
            self.calls.borrow_mut().push(request.clone());
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(HttpTransportError::Unavailable))
        }
    }

    fn policy(max_body: usize, max_pages: usize, max_items: usize) -> HttpClientPolicy {
        HttpClientPolicy::new(
            Duration::from_millis(200),
            Duration::from_millis(200),
            max_body,
            max_pages,
            max_items,
        )
        .unwrap()
    }

    fn fake_client(
        responses: impl IntoIterator<Item = Result<HttpResponse, HttpTransportError>>,
        policy: HttpClientPolicy,
    ) -> HttpDurableStateClient<FakeHttpTransport> {
        HttpDurableStateClient::new(
            "https://sync.example.test",
            AdapterIdentity::from_raw("gdrive-main").unwrap(),
            &SecretString::from_raw("test", "sentinel-adapter-token").unwrap(),
            FakeHttpTransport::with_responses(responses),
            policy,
        )
        .unwrap()
    }

    fn snapshot_json(
        next_after_path: Option<&str>,
        mapping_paths: &[&str],
        state_version: u64,
    ) -> Vec<u8> {
        let mappings: Vec<_> = mapping_paths
            .iter()
            .map(|path| {
                serde_json::json!({
                    "path": path,
                    "drive_file_id": "sentinel-provider-file-id",
                    "echo": { "state": "none" }
                })
            })
            .collect();
        serde_json::to_vec(&serde_json::json!({
            "adapter_id": "gdrive-main",
            "state_format_version": 1,
            "state_version": state_version,
            "cursor": { "generation": 3, "present": true },
            "core_export_checkpoint": 19,
            "last_operations": {},
            "mappings": mappings,
            "next_after_path": next_after_path
        }))
        .unwrap()
    }

    fn minimal_commit() -> GDriveStateCommitRequest {
        serde_json::from_value(serde_json::json!({
            "expected_state_version": 7,
            "cursor": { "expected_generation": 3 },
            "core_export_checkpoint": 19,
            "operation": {
                "operation_id": "op_gdrive_fixture_01",
                "kind": "cursor_checkpoint",
                "facts_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        }))
        .unwrap()
    }

    fn error_body(code: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "error": { "code": code, "message": "safe fixed message" }
        }))
        .unwrap()
    }

    fn spawn_loopback_server(
        handler: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handler(stream);
        });
        (format!("http://{address}"), handle)
    }

    fn read_request_headers(stream: &mut TcpStream) {
        let mut received = Vec::new();
        let mut chunk = [0_u8; 256];
        while !received.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            received.extend_from_slice(&chunk[..count]);
        }
    }

    #[test]
    fn exact_private_get_request_and_snapshot_decoding() {
        let client = fake_client(
            [Ok(HttpResponse::new(
                200,
                snapshot_json(None, &["Notes/a.md"], 7),
            ))],
            policy(8_192, 4, 10),
        );
        let after = VaultPath::parse("Notes/start.md").unwrap();

        let snapshot = client.get_state_page(Some(&after), 5).unwrap();

        assert_eq!(snapshot.state_version, 7);
        let calls = client.transport().calls();
        let request = &calls[0];
        assert_eq!(request.method(), HttpMethod::Get);
        assert_eq!(
            request.expose_url_for_transport(),
            "https://sync.example.test/v1/adapters/gdrive-main/gdrive/state?after_path=Notes%2Fstart.md&limit=5"
        );
        assert_eq!(
            request.authorization_header_for_transport(),
            "Bearer sentinel-adapter-token"
        );
        assert!(request.idempotency_key_for_transport().is_none());
    }

    #[test]
    fn origin_only_server_url_accepts_root_forms_and_rejects_ambiguous_endpoints() {
        for accepted in [
            "https://sync.example.test",
            "https://sync.example.test/",
            "http://127.0.0.1:8080",
            "http://[::1]:8080/",
        ] {
            let client = HttpDurableStateClient::new(
                accepted,
                AdapterIdentity::from_raw("gdrive-main").unwrap(),
                &SecretString::from_raw("test", "token").unwrap(),
                FakeHttpTransport::default(),
                policy(8_192, 4, 10),
            )
            .unwrap();
            let request = client.state_request(None, 1).unwrap();
            assert!(request
                .expose_url_for_transport()
                .contains("/v1/adapters/gdrive-main/gdrive/state?limit=1"));
            assert!(!request.expose_url_for_transport().contains("//v1/"));
        }

        for rejected in [
            "https://sync.example.test/prefix",
            "https://sync.example.test//",
            "https://user@sync.example.test",
            "https://sync.example.test?query=1",
            "https://sync.example.test#fragment",
            "https://",
            "https://:443",
            "https://sync.example.test:",
            "https://sync..example.test",
            "ftp://sync.example.test",
            "https://sync.example.test\\prefix",
        ] {
            let result = HttpDurableStateClient::new(
                rejected,
                AdapterIdentity::from_raw("gdrive-main").unwrap(),
                &SecretString::from_raw("test", "token").unwrap(),
                FakeHttpTransport::default(),
                policy(8_192, 4, 10),
            );
            assert_eq!(
                result.unwrap_err().category(),
                DurableStateErrorCategory::InvalidRequest,
                "{rejected}"
            );
        }
    }

    #[test]
    fn concrete_transport_enforces_overall_deadline_and_classifies_timeout() {
        let (base_url, server) = spawn_loopback_server(|mut stream| {
            read_request_headers(&mut stream);
            thread::sleep(Duration::from_millis(180));
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
        });
        let timeout_policy = HttpClientPolicy::new(
            Duration::from_millis(100),
            Duration::from_millis(40),
            1_024,
            2,
            10,
        )
        .unwrap();
        let transport = UreqHttpTransport::new(timeout_policy);
        let request = HttpRequest {
            method: HttpMethod::Get,
            url: format!("{base_url}/slow"),
            bearer_token: BearerToken::new("sentinel-timeout-token".to_owned()).unwrap(),
            idempotency_key: None,
            content_type: None,
            body: Vec::new(),
        };

        let started = Instant::now();
        let error = transport.execute(&request, 1_024).unwrap_err();
        let elapsed = started.elapsed();

        assert_eq!(error, HttpTransportError::Timeout);
        assert!(elapsed < Duration::from_millis(160));
        server.join().unwrap();
    }

    #[test]
    fn concrete_transport_keeps_response_body_bounded() {
        let (base_url, server) = spawn_loopback_server(|mut stream| {
            read_request_headers(&mut stream);
            let body = vec![b'x'; 256];
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });
        let bounded_policy = HttpClientPolicy::new(
            Duration::from_millis(200),
            Duration::from_millis(200),
            32,
            2,
            10,
        )
        .unwrap();
        let client = HttpDurableStateClient::new(
            &base_url,
            AdapterIdentity::from_raw("gdrive-main").unwrap(),
            &SecretString::from_raw("test", "sentinel-body-token").unwrap(),
            UreqHttpTransport::new(bounded_policy),
            bounded_policy,
        )
        .unwrap();

        let error = client.get_state_page(None, 1).unwrap_err();

        assert_eq!(
            error.category(),
            DurableStateErrorCategory::ResponseTooLarge
        );
        server.join().unwrap();
    }

    #[test]
    fn get_limit_and_denied_modes_fail_before_transport() {
        let client = fake_client([], policy(8_192, 4, 10));
        for limit in [0, MAX_GDRIVE_STATE_ITEMS + 1] {
            assert_eq!(
                client.get_state_page(None, limit).unwrap_err().category(),
                DurableStateErrorCategory::InvalidRequest
            );
        }
        assert!(client.transport().calls().is_empty());

        let disabled = ModeAwareDurableStateClient::new(
            AdapterMode::Disabled,
            fake_client([], policy(8_192, 4, 10)),
        );
        assert_eq!(
            disabled.get_state_page(None, 1).unwrap_err().category(),
            DurableStateErrorCategory::ModeDenied
        );
        assert!(disabled.inner().transport().calls().is_empty());

        let read_only = ModeAwareDurableStateClient::new(
            AdapterMode::ReadOnly,
            fake_client([], policy(8_192, 4, 10)),
        );
        assert_eq!(
            read_only
                .compare_and_commit(&IdempotencyKey::new("safe-key").unwrap(), &minimal_commit(),)
                .unwrap_err()
                .category(),
            DurableStateErrorCategory::ModeDenied
        );
        assert!(read_only.inner().transport().calls().is_empty());
    }

    #[test]
    fn exact_post_contract_and_all_accepted_outcomes_decode() {
        for (status, body) in [
            (
                200,
                r#"{"status":"committed","state_version":8,"cursor_generation":4,"core_export_checkpoint":20}"#,
            ),
            (200, r#"{"status":"replayed","state_version":8}"#),
            (409, r#"{"status":"stale_state"}"#),
            (409, r#"{"status":"cursor_regression"}"#),
            (409, r#"{"status":"cursor_gap"}"#),
            (409, r#"{"status":"mapping_conflict"}"#),
            (409, r#"{"status":"idempotency_conflict"}"#),
            (422, r#"{"status":"validation_failed"}"#),
        ] {
            let client = fake_client(
                [Ok(HttpResponse::new(status, body.as_bytes()))],
                policy(8_192, 4, 10),
            );
            let key = IdempotencyKey::new("sentinel-idempotency-key").unwrap();
            let commit = minimal_commit();
            let outcome = client.compare_and_commit(&key, &commit).unwrap();
            assert_eq!(
                serde_json::to_value(outcome).unwrap(),
                serde_json::from_str::<serde_json::Value>(body).unwrap()
            );
            let calls = client.transport().calls();
            let request = &calls[0];
            assert_eq!(request.method(), HttpMethod::Post);
            assert_eq!(
                request.expose_url_for_transport(),
                "https://sync.example.test/v1/adapters/gdrive-main/gdrive/state/commit"
            );
            assert_eq!(
                request.idempotency_key_for_transport(),
                Some("sentinel-idempotency-key")
            );
            assert_eq!(
                request.content_type_for_transport(),
                Some("application/json")
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(request.body_for_transport()).unwrap(),
                serde_json::to_value(&commit).unwrap()
            );
        }
    }

    #[test]
    fn route_errors_have_safe_retry_classification() {
        for (status, code, category, retryable) in [
            (
                401,
                "unauthorized",
                DurableStateErrorCategory::Unauthorized,
                false,
            ),
            (
                403,
                "forbidden",
                DurableStateErrorCategory::Forbidden,
                false,
            ),
            (
                404,
                "adapter_not_found",
                DurableStateErrorCategory::NotFound,
                false,
            ),
            (
                409,
                "invalid_cursor_state",
                DurableStateErrorCategory::InvalidCursorState,
                false,
            ),
            (
                422,
                "validation_error",
                DurableStateErrorCategory::Validation,
                false,
            ),
            (500, "internal", DurableStateErrorCategory::Internal, true),
            (
                503,
                "unavailable",
                DurableStateErrorCategory::ServiceUnavailable,
                true,
            ),
        ] {
            let client = fake_client(
                [Ok(HttpResponse::new(status, error_body(code)))],
                policy(8_192, 4, 10),
            );
            let error = client.get_state_page(None, 1).unwrap_err();
            assert_eq!(error.category(), category);
            assert_eq!(error.is_retryable(), retryable);
            assert!(!error.to_string().contains("safe fixed message"));
        }
    }

    #[test]
    fn pagination_rejects_loops_bounds_and_inconsistent_snapshots() {
        let looped = fake_client(
            [
                Ok(HttpResponse::new(
                    200,
                    snapshot_json(Some("Notes/a.md"), &["Notes/0.md"], 7),
                )),
                Ok(HttpResponse::new(
                    200,
                    snapshot_json(Some("Notes/a.md"), &["Notes/a.md"], 7),
                )),
            ],
            policy(8_192, 4, 10),
        );
        assert_eq!(
            looped.collect_state(1).unwrap_err().category(),
            DurableStateErrorCategory::PaginationLoop
        );

        let inconsistent = fake_client(
            [
                Ok(HttpResponse::new(
                    200,
                    snapshot_json(Some("Notes/a.md"), &["Notes/0.md"], 7),
                )),
                Ok(HttpResponse::new(
                    200,
                    snapshot_json(None, &["Notes/a.md"], 8),
                )),
            ],
            policy(8_192, 4, 10),
        );
        assert_eq!(
            inconsistent.collect_state(1).unwrap_err().category(),
            DurableStateErrorCategory::InconsistentSnapshot
        );

        let item_limited = fake_client(
            [Ok(HttpResponse::new(
                200,
                snapshot_json(None, &["Notes/a.md", "Notes/b.md"], 7),
            ))],
            policy(8_192, 4, 1),
        );
        assert_eq!(
            item_limited.collect_state(2).unwrap_err().category(),
            DurableStateErrorCategory::CollectionTooLarge
        );
    }

    #[test]
    fn client_request_response_collection_and_errors_never_format_private_sentinels() {
        let client = fake_client([], policy(8_192, 4, 10));
        let request = client
            .commit_request(
                &IdempotencyKey::new("sentinel-idempotency-key").unwrap(),
                &minimal_commit(),
            )
            .unwrap();
        let response = HttpResponse::new(500, b"sentinel-private-response-body".as_slice());
        let collection = CollectedGDriveState {
            state_format_version: 1,
            state_version: 7,
            cursor_generation: 3,
            core_export_checkpoint: 19,
            last_operations: serde_json::from_value(serde_json::json!({})).unwrap(),
            mappings: serde_json::from_value(serde_json::json!([{
                "path": "Sentinel/private-path.md",
                "drive_file_id": "sentinel-provider-file-id",
                "echo": { "state": "none" }
            }]))
            .unwrap(),
            page_count: 1,
        };
        let error = DurableStateClientError::new(DurableStateErrorCategory::Internal);
        let rendered = format!(
            "{client:?} {request:?} {request} {response:?} {collection:?} {error:?} {error}"
        );
        for sentinel in [
            "sentinel-adapter-token",
            "gdrive-main",
            "sentinel-idempotency-key",
            "Sentinel/private-path.md",
            "sentinel-provider-file-id",
            "sentinel-private-response-body",
            "op_gdrive_fixture_01",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert!(!rendered.contains(sentinel));
        }
    }
}
