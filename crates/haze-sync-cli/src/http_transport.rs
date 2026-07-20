//! Bounded authenticated HTTP transport for accepted Server surfaces.

use crate::{
    config::ServerUrl,
    config_loader::{BearerTokenProvider, SecretToken, TokenLoadError},
    doctor_live::{
        DoctorReadClient, HealthSummary, ReadinessComponentState, ReadinessOverallStatus,
        ReadinessSummary,
    },
    server_api::{
        AdapterCursorSummary, AdapterList, AdapterSummary, DependencyReadinessState,
        PauseStatusSummary, ServerReadClient, ServerReadError, ServerStatus, StatusSummary,
    },
    worktree_api::{
        WorktreeClient, WorktreeClientError, WorktreeStatusRequest, WorktreeSyncRequest,
    },
};
use haze_sync_api::{
    contracts::errors::{ErrorResponse, PublicErrorCode},
    dto::worktree::{WorktreeStatusResponse, WorktreeSyncOnceResponse},
    routes::admin as api_admin,
};
use rustls::{
    ClientConfig, ClientConnection, OwnedTrustAnchor, RootCertStore, ServerName, StreamOwned,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::{
    fmt,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use url::Url;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_CHUNK_OVERHEAD_BYTES: usize = 64 * 1024;

pub struct AuthenticatedHttpClient<T> {
    token_provider: T,
    options: HttpTransportOptions,
}

impl<T> AuthenticatedHttpClient<T> {
    #[must_use]
    pub fn new(token_provider: T) -> Self {
        Self {
            token_provider,
            options: HttpTransportOptions::default(),
        }
    }

    #[cfg(test)]
    fn with_options(token_provider: T, options: HttpTransportOptions) -> Self {
        Self {
            token_provider,
            options,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HttpTransportOptions {
    connect_timeout: Duration,
    io_timeout: Duration,
    max_response_bytes: usize,
}

impl Default for HttpTransportOptions {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            io_timeout: DEFAULT_IO_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl<T> AuthenticatedHttpClient<T>
where
    T: BearerTokenProvider,
{
    fn request_json(
        &self,
        server_url: &ServerUrl,
        method: &'static str,
        path: &'static str,
        body: Option<&'static str>,
    ) -> Result<HttpResponse, TransportError> {
        let target = RequestTarget::parse(server_url, path)?;
        let token = self
            .token_provider
            .bearer_token()
            .map_err(TransportError::Token)?;
        let request = build_request(&target, method, body, token.as_ref());
        let stream = connect(&target, self.options)?;

        match target.scheme {
            Scheme::Http => execute_on_stream(stream, &request, self.options),
            Scheme::Https => {
                let server_name = ServerName::try_from(target.host.as_str())
                    .map_err(|_| TransportError::InvalidServerUrl)?;
                let connection = ClientConnection::new(tls_config(), server_name)
                    .map_err(|_| TransportError::Tls)?;
                execute_on_stream(StreamOwned::new(connection, stream), &request, self.options)
            }
        }
    }
}

impl<T> ServerReadClient for AuthenticatedHttpClient<T>
where
    T: BearerTokenProvider,
{
    fn fetch_status(&self, server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
        let response = self
            .request_json(server_url, "GET", "/v1/admin/status", None)
            .map_err(server_transport_error)?;
        if response.status != 200 {
            return Err(server_http_error(&response));
        }
        let value: api_admin::StatusSummaryResponse = decode_json(&response.body)?;
        Ok(StatusSummary {
            server_status: match value.server_status {
                api_admin::ServerStatus::NotReady => ServerStatus::NotReady,
                api_admin::ServerStatus::Ready => ServerStatus::Ready,
                api_admin::ServerStatus::Degraded => ServerStatus::Degraded,
                api_admin::ServerStatus::Maintenance => ServerStatus::Maintenance,
            },
            db_readiness_state: convert_dependency(value.db_readiness_state),
            object_store_readiness_state: convert_dependency(value.object_store_readiness_state),
            last_operation_sequence: value.last_operation_sequence,
            adapter_count: value.adapter_count,
            pause: PauseStatusSummary {
                supported: value.pause.supported,
                active: value.pause.active,
            },
        })
    }

    fn fetch_adapters(&self, server_url: &ServerUrl) -> Result<AdapterList, ServerReadError> {
        let response = self
            .request_json(server_url, "GET", "/v1/admin/adapters", None)
            .map_err(server_transport_error)?;
        if response.status != 200 {
            return Err(server_http_error(&response));
        }
        let value: api_admin::AdapterListResponse = decode_json(&response.body)?;
        let adapters = value
            .adapters
            .into_iter()
            .map(|adapter| AdapterSummary {
                adapter_id: adapter.adapter_id.to_string(),
                role: adapter.role.map(|role| role.to_string()),
                mode: adapter.mode.map(|mode| mode.to_string()),
                enabled: adapter.enabled,
                last_seen_at: adapter.last_seen_at.map(|value| value.to_string()),
                cursor: adapter.cursor.map(|cursor| AdapterCursorSummary {
                    last_core_seq: cursor.last_core_seq,
                    last_success_at: cursor.last_success_at.map(|value| value.to_string()),
                    has_external_cursor: cursor.has_external_cursor,
                }),
            })
            .collect::<Vec<_>>();
        if value.total_count != adapters.len() as u64 {
            return Err(ServerReadError::InvalidResponse);
        }
        Ok(AdapterList::new(adapters))
    }
}

impl<T> DoctorReadClient for AuthenticatedHttpClient<T>
where
    T: BearerTokenProvider,
{
    fn fetch_health(&self, server_url: &ServerUrl) -> Result<HealthSummary, ServerReadError> {
        let response = self
            .request_json(server_url, "GET", "/health", None)
            .map_err(server_transport_error)?;
        if response.status != 200 {
            return Err(server_http_error(&response));
        }
        let value: HealthWire = decode_json(&response.body)?;
        Ok(HealthSummary {
            process_alive: value.status == "ok",
        })
    }

    fn fetch_readiness(
        &self,
        server_url: &ServerUrl,
    ) -> Result<ReadinessSummary, ServerReadError> {
        let response = self
            .request_json(server_url, "GET", "/ready", None)
            .map_err(server_transport_error)?;
        if !matches!(response.status, 200 | 503) {
            return Err(server_http_error(&response));
        }
        let value: ReadinessWire = decode_json(&response.body)?;
        let mut database = ReadinessComponentState::Unknown;
        let mut object_store = ReadinessComponentState::Unknown;
        for component in value.components {
            let state = match component.status {
                ReadinessComponentWire::Ready => ReadinessComponentState::Ready,
                ReadinessComponentWire::NotReady => ReadinessComponentState::NotReady,
                ReadinessComponentWire::Disabled => ReadinessComponentState::Disabled,
            };
            match component.name.as_str() {
                "database" => database = state,
                "object_store" => object_store = state,
                _ => {}
            }
        }
        let overall = match value.status {
            ReadinessOverallWire::Ready => ReadinessOverallStatus::Ready,
            ReadinessOverallWire::NotReady => ReadinessOverallStatus::NotReady,
        };
        if (response.status == 200) != (overall == ReadinessOverallStatus::Ready) {
            return Err(ServerReadError::InvalidResponse);
        }
        Ok(ReadinessSummary {
            overall,
            database,
            object_store,
        })
    }

    fn fetch_status(&self, server_url: &ServerUrl) -> Result<StatusSummary, ServerReadError> {
        ServerReadClient::fetch_status(self, server_url)
    }
}

impl<T> WorktreeClient for AuthenticatedHttpClient<T>
where
    T: BearerTokenProvider,
{
    fn fetch_worktree_status(
        &self,
        server_url: &ServerUrl,
        request: WorktreeStatusRequest,
    ) -> Result<(u16, WorktreeStatusResponse), WorktreeClientError> {
        let response = self
            .request_json(server_url, request.method(), request.path(), None)
            .map_err(worktree_transport_error)?;
        if response.status != 200 {
            return Err(worktree_http_error(&response));
        }
        let body = serde_json::from_slice(&response.body)
            .map_err(|_| WorktreeClientError::InvalidResponse)?;
        Ok((response.status, body))
    }

    fn submit_worktree_sync_once(
        &self,
        server_url: &ServerUrl,
        request: WorktreeSyncRequest,
    ) -> Result<(u16, WorktreeSyncOnceResponse), WorktreeClientError> {
        let response = self
            .request_json(
                server_url,
                request.method(),
                request.path(),
                Some(request.body_json()),
            )
            .map_err(worktree_transport_error)?;
        if !matches!(response.status, 202 | 409 | 500 | 503) {
            return Err(worktree_http_error(&response));
        }
        match serde_json::from_slice(&response.body) {
            Ok(body) => Ok((response.status, body)),
            Err(_) => Err(worktree_http_error(&response)),
        }
    }
}

fn convert_dependency(value: api_admin::DependencyReadinessState) -> DependencyReadinessState {
    match value {
        api_admin::DependencyReadinessState::Unknown => DependencyReadinessState::Unknown,
        api_admin::DependencyReadinessState::Ready => DependencyReadinessState::Ready,
        api_admin::DependencyReadinessState::NotReady => DependencyReadinessState::NotReady,
    }
}

fn decode_json<T>(body: &[u8]) -> Result<T, ServerReadError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(body).map_err(|_| ServerReadError::InvalidResponse)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HealthWire {
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadinessOverallWire {
    Ready,
    NotReady,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadinessComponentWire {
    Ready,
    NotReady,
    Disabled,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessComponentResponseWire {
    name: String,
    status: ReadinessComponentWire,
    #[serde(rename = "code")]
    _code: String,
    #[serde(rename = "message")]
    _message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessWire {
    status: ReadinessOverallWire,
    components: Vec<ReadinessComponentResponseWire>,
}

fn server_transport_error(error: TransportError) -> ServerReadError {
    match error {
        TransportError::Token(TokenLoadError::EmptyToken)
        | TransportError::Token(TokenLoadError::InvalidTokenEncoding) => {
            ServerReadError::Unauthorized
        }
        TransportError::Token(_)
        | TransportError::Dns
        | TransportError::Connect
        | TransportError::Timeout
        | TransportError::Io
        | TransportError::Tls => ServerReadError::ServerUnavailable,
        TransportError::InvalidServerUrl
        | TransportError::RedirectRefused
        | TransportError::ResponseTooLarge
        | TransportError::MalformedResponse => ServerReadError::InvalidResponse,
    }
}

fn worktree_transport_error(error: TransportError) -> WorktreeClientError {
    match server_transport_error(error) {
        ServerReadError::Unauthorized => WorktreeClientError::Unauthorized,
        ServerReadError::Forbidden => WorktreeClientError::Forbidden,
        ServerReadError::ServerUnavailable | ServerReadError::NotReady => {
            WorktreeClientError::ServerUnavailable
        }
        ServerReadError::InvalidResponse
        | ServerReadError::NotConfigured
        | ServerReadError::Offline => WorktreeClientError::InvalidResponse,
    }
}

fn server_http_error(response: &HttpResponse) -> ServerReadError {
    if let Ok(error) = serde_json::from_slice::<ErrorResponse>(&response.body) {
        return match error.error.code {
            PublicErrorCode::Unauthorized
            | PublicErrorCode::MissingToken
            | PublicErrorCode::InvalidToken => ServerReadError::Unauthorized,
            PublicErrorCode::ForbiddenRole => ServerReadError::Forbidden,
            PublicErrorCode::InternalError => ServerReadError::ServerUnavailable,
            _ => ServerReadError::InvalidResponse,
        };
    }
    match response.status {
        401 => ServerReadError::Unauthorized,
        403 => ServerReadError::Forbidden,
        503 | 500 => ServerReadError::ServerUnavailable,
        _ => ServerReadError::InvalidResponse,
    }
}

fn worktree_http_error(response: &HttpResponse) -> WorktreeClientError {
    match server_http_error(response) {
        ServerReadError::Unauthorized => WorktreeClientError::Unauthorized,
        ServerReadError::Forbidden => WorktreeClientError::Forbidden,
        ServerReadError::ServerUnavailable | ServerReadError::NotReady => {
            WorktreeClientError::ServerUnavailable
        }
        _ => WorktreeClientError::InvalidResponse,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Scheme {
    Http,
    Https,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RequestTarget {
    scheme: Scheme,
    host: String,
    port: u16,
    path: String,
    host_header: String,
}

impl RequestTarget {
    fn parse(server_url: &ServerUrl, endpoint: &str) -> Result<Self, TransportError> {
        let base = Url::parse(server_url.as_str()).map_err(|_| TransportError::InvalidServerUrl)?;
        if !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !matches!(base.path(), "" | "/")
        {
            return Err(TransportError::InvalidServerUrl);
        }
        let scheme = match base.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            _ => return Err(TransportError::InvalidServerUrl),
        };
        let host = base
            .host_str()
            .filter(|host| !host.is_empty())
            .ok_or(TransportError::InvalidServerUrl)?
            .to_owned();
        let port = base
            .port_or_known_default()
            .ok_or(TransportError::InvalidServerUrl)?;
        let default_port = matches!((scheme, port), (Scheme::Http, 80) | (Scheme::Https, 443));
        let display_host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host.clone()
        };
        let host_header = if default_port {
            display_host
        } else {
            format!("{display_host}:{port}")
        };
        if !endpoint.starts_with('/')
            || endpoint.contains(|character| character == '\r' || character == '\n')
        {
            return Err(TransportError::InvalidServerUrl);
        }
        Ok(Self {
            scheme,
            host,
            port,
            path: endpoint.to_owned(),
            host_header,
        })
    }
}

fn build_request(
    target: &RequestTarget,
    method: &str,
    body: Option<&str>,
    token: Option<&SecretToken>,
) -> Vec<u8> {
    let body = body.unwrap_or("");
    let mut request = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\n",
        target.path, target.host_header
    );
    if let Some(token) = token {
        request.push_str("Authorization: Bearer ");
        request.push_str(token.as_str());
        request.push_str("\r\n");
    }
    if !body.is_empty() || method == "POST" {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    let mut bytes = request.into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

fn resolve_addresses(
    host: String,
    port: u16,
    timeout: Duration,
) -> Result<Vec<SocketAddr>, TransportError> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("haze-sync-cli-dns".to_owned())
        .spawn(move || {
            let result = (host.as_str(), port)
                .to_socket_addrs()
                .map(|addresses| addresses.collect::<Vec<_>>());
            let _ = sender.send(result);
        })
        .map_err(|_| TransportError::Dns)?;
    match receiver.recv_timeout(timeout) {
        Ok(Ok(addresses)) if !addresses.is_empty() => Ok(addresses),
        Ok(_) => Err(TransportError::Dns),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(TransportError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(TransportError::Dns),
    }
}

fn connect(
    target: &RequestTarget,
    options: HttpTransportOptions,
) -> Result<TcpStream, TransportError> {
    let started = Instant::now();
    let addresses = resolve_addresses(target.host.clone(), target.port, options.connect_timeout)?;
    let mut last_error = None;
    for address in addresses {
        let remaining = options
            .connect_timeout
            .checked_sub(started.elapsed())
            .ok_or(TransportError::Timeout)?;
        match TcpStream::connect_timeout(&address, remaining) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(options.io_timeout))
                    .map_err(|_| TransportError::Io)?;
                stream
                    .set_write_timeout(Some(options.io_timeout))
                    .map_err(|_| TransportError::Io)?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    match last_error {
        Some(error) if is_timeout(&error) => Err(TransportError::Timeout),
        Some(_) | None => Err(TransportError::Connect),
    }
}

fn tls_config() -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|anchor| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            anchor.subject,
            anchor.spki,
            anchor.name_constraints,
        )
    }));
    Arc::new(
        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

fn execute_on_stream<S>(
    mut stream: S,
    request: &[u8],
    options: HttpTransportOptions,
) -> Result<HttpResponse, TransportError>
where
    S: Read + Write,
{
    stream.write_all(request).map_err(map_io_error)?;
    stream.flush().map_err(map_io_error)?;

    let max_wire_bytes = MAX_HEADER_BYTES
        .saturating_add(options.max_response_bytes)
        .saturating_add(MAX_CHUNK_OVERHEAD_BYTES);
    let mut wire = Vec::new();
    stream
        .take((max_wire_bytes + 1) as u64)
        .read_to_end(&mut wire)
        .map_err(map_io_error)?;
    if wire.len() > max_wire_bytes {
        return Err(TransportError::ResponseTooLarge);
    }
    parse_response(&wire, options.max_response_bytes)
}

fn parse_response(wire: &[u8], max_body_bytes: usize) -> Result<HttpResponse, TransportError> {
    let header_end =
        find_subsequence(wire, b"\r\n\r\n").ok_or(TransportError::MalformedResponse)?;
    if header_end > MAX_HEADER_BYTES {
        return Err(TransportError::ResponseTooLarge);
    }
    let header = std::str::from_utf8(&wire[..header_end])
        .map_err(|_| TransportError::MalformedResponse)?;
    let mut lines = header.split("\r\n");
    let status_line = lines.next().ok_or(TransportError::MalformedResponse)?;
    let mut status_parts = status_line.split_whitespace();
    let version = status_parts
        .next()
        .ok_or(TransportError::MalformedResponse)?;
    let status = status_parts
        .next()
        .ok_or(TransportError::MalformedResponse)?
        .parse::<u16>()
        .map_err(|_| TransportError::MalformedResponse)?;
    if !matches!(version, "HTTP/1.0" | "HTTP/1.1") || !(100..=599).contains(&status) {
        return Err(TransportError::MalformedResponse);
    }
    if (300..400).contains(&status) {
        return Err(TransportError::RedirectRefused);
    }

    let mut content_length = None;
    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(TransportError::MalformedResponse);
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-length" => {
                let length = value
                    .parse::<usize>()
                    .map_err(|_| TransportError::MalformedResponse)?;
                if content_length.replace(length).is_some() {
                    return Err(TransportError::MalformedResponse);
                }
            }
            "transfer-encoding" => {
                if !value.eq_ignore_ascii_case("chunked") {
                    return Err(TransportError::MalformedResponse);
                }
                chunked = true;
            }
            "content-encoding" if !value.eq_ignore_ascii_case("identity") => {
                return Err(TransportError::MalformedResponse);
            }
            _ => {}
        }
    }
    if chunked && content_length.is_some() {
        return Err(TransportError::MalformedResponse);
    }

    let raw_body = &wire[header_end + 4..];
    let body = if chunked {
        decode_chunked(raw_body, max_body_bytes)?
    } else if let Some(length) = content_length {
        if length > max_body_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        if raw_body.len() != length {
            return Err(TransportError::MalformedResponse);
        }
        raw_body.to_vec()
    } else {
        if raw_body.len() > max_body_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        raw_body.to_vec()
    };

    Ok(HttpResponse { status, body })
}

fn decode_chunked(wire: &[u8], max_body_bytes: usize) -> Result<Vec<u8>, TransportError> {
    let mut offset = 0;
    let mut body = Vec::new();
    loop {
        let relative_end = find_subsequence(&wire[offset..], b"\r\n")
            .ok_or(TransportError::MalformedResponse)?;
        let line_end = offset + relative_end;
        let size_line = std::str::from_utf8(&wire[offset..line_end])
            .map_err(|_| TransportError::MalformedResponse)?;
        let size_text = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| TransportError::MalformedResponse)?;
        offset = line_end + 2;
        if size == 0 {
            if wire.get(offset..offset + 2) != Some(b"\r\n") {
                return Err(TransportError::MalformedResponse);
            }
            return Ok(body);
        }
        if body.len().saturating_add(size) > max_body_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        let chunk_end = offset
            .checked_add(size)
            .ok_or(TransportError::ResponseTooLarge)?;
        let chunk = wire
            .get(offset..chunk_end)
            .ok_or(TransportError::MalformedResponse)?;
        if wire.get(chunk_end..chunk_end + 2) != Some(b"\r\n") {
            return Err(TransportError::MalformedResponse);
        }
        body.extend_from_slice(chunk);
        offset = chunk_end + 2;
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn map_io_error(error: io::Error) -> TransportError {
    if is_timeout(&error) {
        TransportError::Timeout
    } else {
        TransportError::Io
    }
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TransportError {
    InvalidServerUrl,
    Token(TokenLoadError),
    Dns,
    Connect,
    Timeout,
    Io,
    Tls,
    RedirectRefused,
    ResponseTooLarge,
    MalformedResponse,
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded HTTP transport failed")
    }
}

impl std::error::Error for TransportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_loader::SecretToken;
    use std::{
        net::TcpListener,
        sync::{Arc, Mutex},
    };

    #[derive(Clone)]
    struct FixedToken(Option<SecretToken>);

    impl BearerTokenProvider for FixedToken {
        fn bearer_token(&self) -> Result<Option<SecretToken>, TokenLoadError> {
            Ok(self.0.clone())
        }
    }

    fn fixed_token(value: Option<&str>) -> FixedToken {
        FixedToken(value.map(|value| SecretToken::parse(value).unwrap()))
    }

    fn spawn_server(response: Vec<u8>, delay: Duration) -> (ServerUrl, Arc<Mutex<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let request = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&request);
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0_u8; 4096];
            let count = stream.read(&mut bytes).unwrap_or(0);
            captured.lock().unwrap().extend_from_slice(&bytes[..count]);
            thread::sleep(delay);
            let _ = stream.write_all(&response);
        });
        (
            ServerUrl::parse(&format!("http://{address}")).unwrap(),
            request,
        )
    }

    fn response(status: &str, body: &str, extra_headers: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn test_options() -> HttpTransportOptions {
        HttpTransportOptions {
            connect_timeout: Duration::from_millis(300),
            io_timeout: Duration::from_millis(300),
            max_response_bytes: 2048,
        }
    }

    #[test]
    fn authenticated_status_request_uses_bearer_header_and_decodes_contract() {
        let body = "{\"server_status\":\"ready\",\"db_readiness_state\":\"ready\",\"object_store_readiness_state\":\"ready\",\"last_operation_sequence\":42,\"adapter_count\":1,\"pause\":{\"supported\":false,\"active\":null}}";
        let (url, captured) = spawn_server(response("200 OK", body, ""), Duration::ZERO);
        let client = AuthenticatedHttpClient::with_options(
            fixed_token(Some("route_test_token")),
            test_options(),
        );

        let status = ServerReadClient::fetch_status(&client, &url).unwrap();
        assert_eq!(status.server_status, ServerStatus::Ready);
        let request = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(request.starts_with("GET /v1/admin/status HTTP/1.1"));
        assert!(request.contains("Authorization: Bearer route_test_token\r\n"));
    }

    #[test]
    fn redirect_is_refused_without_following_location() {
        let (url, _) = spawn_server(
            response(
                "302 Found",
                "{}",
                "Location: http://127.0.0.1:9/private\r\n",
            ),
            Duration::ZERO,
        );
        let client = AuthenticatedHttpClient::with_options(fixed_token(None), test_options());
        assert_eq!(
            ServerReadClient::fetch_status(&client, &url).unwrap_err(),
            ServerReadError::InvalidResponse
        );
    }

    #[test]
    fn malformed_and_oversized_responses_are_rejected() {
        let (malformed_url, _) = spawn_server(b"not-http".to_vec(), Duration::ZERO);
        let client = AuthenticatedHttpClient::with_options(fixed_token(None), test_options());
        assert_eq!(
            ServerReadClient::fetch_status(&client, &malformed_url).unwrap_err(),
            ServerReadError::InvalidResponse
        );

        let oversized = "x".repeat(4096);
        let (oversized_url, _) =
            spawn_server(response("200 OK", &oversized, ""), Duration::ZERO);
        assert_eq!(
            ServerReadClient::fetch_status(&client, &oversized_url).unwrap_err(),
            ServerReadError::InvalidResponse
        );
    }

    #[test]
    fn timeout_and_unavailable_server_are_safe_runtime_errors() {
        let body = "{\"server_status\":\"ready\",\"db_readiness_state\":\"ready\",\"object_store_readiness_state\":\"ready\",\"last_operation_sequence\":null,\"adapter_count\":0,\"pause\":{\"supported\":false,\"active\":null}}";
        let (url, _) = spawn_server(
            response("200 OK", body, ""),
            Duration::from_millis(600),
        );
        let client = AuthenticatedHttpClient::with_options(fixed_token(None), test_options());
        assert_eq!(
            ServerReadClient::fetch_status(&client, &url).unwrap_err(),
            ServerReadError::ServerUnavailable
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let unavailable = ServerUrl::parse(&format!("http://{address}")).unwrap();
        assert_eq!(
            ServerReadClient::fetch_status(&client, &unavailable).unwrap_err(),
            ServerReadError::ServerUnavailable
        );
    }

    #[test]
    fn common_http_failures_map_without_exposing_server_body() {
        for (status, expected) in [
            ("401 Unauthorized", ServerReadError::Unauthorized),
            ("403 Forbidden", ServerReadError::Forbidden),
            (
                "503 Service Unavailable",
                ServerReadError::ServerUnavailable,
            ),
        ] {
            let (url, _) = spawn_server(response(status, "{}", ""), Duration::ZERO);
            let client = AuthenticatedHttpClient::with_options(fixed_token(None), test_options());
            assert_eq!(
                ServerReadClient::fetch_status(&client, &url),
                Err(expected)
            );
        }
    }
}
