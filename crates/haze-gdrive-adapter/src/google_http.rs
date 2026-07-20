//! Bounded concrete Google OAuth and Drive v3 HTTP clients.
//!
//! This module implements provider transport only. It does not own durable-state
//! orchestration, scheduling, Core policy, or long-running runtime composition.

use crate::auth::{
    AccessToken, AuthError, AuthErrorCategory, GoogleAuthClient, GoogleProviderClient,
    TokenEndpoint, TokenRefreshRequest,
};
use crate::change_feed::{
    DriveChangeEntry, DriveChangeFeedProvider, DriveChangePage, DriveChangePoll,
};
use crate::config::SecretString;
use crate::drive::{
    classify_drive_metadata, DriveEntryClassification, DriveEntryKind, DriveMetadata,
    DriveMutationOutcome, DriveProvider, DriveUpdateRequest, DriveUploadRequest, ProviderError,
    ProviderErrorCategory, MIME_GOOGLE_DOC, MIME_GOOGLE_FOLDER, MIME_GOOGLE_SHEET,
    MIME_GOOGLE_SHORTCUT, MIME_GOOGLE_SLIDE,
};
use serde_json::{Map, Value};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt;
use std::io::{self, Read};
use std::time::{Duration, SystemTime};

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const CHANGES_URL: &str = "https://www.googleapis.com/drive/v3/changes";
const START_TOKEN_URL: &str = "https://www.googleapis.com/drive/v3/changes/startPageToken";
const MULTIPART_BOUNDARY: &str = "haze_sync_drive_boundary_7ab3f7e1";
const METADATA_FIELDS: &str =
    "id,name,mimeType,parents,modifiedTime,size,md5Checksum,version,trashed,driveId";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoogleHttpMethod {
    Get,
    Post,
    Patch,
}

#[derive(Clone)]
pub struct GoogleHttpRequest {
    method: GoogleHttpMethod,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl GoogleHttpRequest {
    fn new(method: GoogleHttpMethod, url: String) -> Self {
        Self {
            method,
            url,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn header(mut self, name: &str, value: String) -> Self {
        self.headers.push((name.to_owned(), value));
        self
    }

    fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    #[must_use]
    pub const fn method(&self) -> GoogleHttpMethod {
        self.method
    }

    #[must_use]
    pub fn url_for_transport(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn headers_for_transport(&self) -> &[(String, String)] {
        &self.headers
    }

    #[must_use]
    pub fn body_for_transport(&self) -> &[u8] {
        &self.body
    }
}

impl fmt::Debug for GoogleHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GoogleHttpRequest")
            .field("method", &self.method)
            .field("url", &"<redacted-google-url>")
            .field("headers", &"<redacted-google-headers>")
            .field("body", &"<redacted-google-body>")
            .finish()
    }
}

impl fmt::Display for GoogleHttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Google HTTP request (<redacted>)")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct GoogleHttpResponse {
    status: u16,
    body: Vec<u8>,
}

impl GoogleHttpResponse {
    #[must_use]
    pub fn new(status: u16, body: impl AsRef<[u8]>) -> Self {
        Self {
            status,
            body: body.as_ref().to_vec(),
        }
    }

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

impl fmt::Debug for GoogleHttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GoogleHttpResponse")
            .field("status", &self.status)
            .field("body", &"<redacted-google-response>")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoogleHttpTransportError {
    Timeout,
    Unavailable,
}

impl fmt::Display for GoogleHttpTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Timeout => "Google HTTP transport timed out",
            Self::Unavailable => "Google HTTP transport is unavailable",
        })
    }
}

impl Error for GoogleHttpTransportError {}

pub trait GoogleHttpTransport {
    fn execute(
        &self,
        request: &GoogleHttpRequest,
        max_response_bytes: usize,
    ) -> Result<GoogleHttpResponse, GoogleHttpTransportError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoogleHttpPolicy {
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_json_bytes: usize,
    pub max_download_bytes: usize,
    pub max_upload_bytes: usize,
    pub max_pages: usize,
    pub max_items: usize,
}

impl Default for GoogleHttpPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(20),
            max_json_bytes: 1_048_576,
            max_download_bytes: 64 * 1_048_576,
            max_upload_bytes: 64 * 1_048_576,
            max_pages: 128,
            max_items: 10_000,
        }
    }
}

pub struct UreqGoogleHttpTransport {
    agent: ureq::Agent,
}

impl UreqGoogleHttpTransport {
    #[must_use]
    pub fn new(policy: GoogleHttpPolicy) -> Self {
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

impl fmt::Debug for UreqGoogleHttpTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UreqGoogleHttpTransport(redirects=disabled,bounded=true)")
    }
}

impl GoogleHttpTransport for UreqGoogleHttpTransport {
    fn execute(
        &self,
        request: &GoogleHttpRequest,
        max_response_bytes: usize,
    ) -> Result<GoogleHttpResponse, GoogleHttpTransportError> {
        let mut outbound = match request.method() {
            GoogleHttpMethod::Get => self.agent.get(request.url_for_transport()),
            GoogleHttpMethod::Post => self.agent.post(request.url_for_transport()),
            GoogleHttpMethod::Patch => self.agent.request("PATCH", request.url_for_transport()),
        };
        for (name, value) in request.headers_for_transport() {
            outbound = outbound.set(name, value);
        }
        let result = if request.body_for_transport().is_empty() {
            outbound.call()
        } else {
            outbound.send_bytes(request.body_for_transport())
        };
        let response = match result {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(error)) => {
                let timed_out = error.kind() == ureq::ErrorKind::Io
                    && error
                        .message()
                        .is_some_and(|message| message.to_ascii_lowercase().contains("timed out"));
                return Err(if timed_out {
                    GoogleHttpTransportError::Timeout
                } else {
                    GoogleHttpTransportError::Unavailable
                });
            }
        };
        let status = response.status();
        let mut body = Vec::new();
        response
            .into_reader()
            .take(max_response_bytes.saturating_add(1) as u64)
            .read_to_end(&mut body)
            .map_err(map_io_error)?;
        Ok(GoogleHttpResponse::new(status, body))
    }
}

fn map_io_error(error: io::Error) -> GoogleHttpTransportError {
    if matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ) {
        GoogleHttpTransportError::Timeout
    } else {
        GoogleHttpTransportError::Unavailable
    }
}

pub trait GoogleClock {
    fn now(&self) -> SystemTime;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemGoogleClock;

impl GoogleClock for SystemGoogleClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GoogleOAuthConfig {
    client_id: SecretString,
    client_secret: SecretString,
    refresh_token: Option<SecretString>,
    scopes: BTreeSet<String>,
}

impl GoogleOAuthConfig {
    pub fn new(
        client_id: SecretString,
        client_secret: SecretString,
        refresh_token: Option<SecretString>,
        scopes: BTreeSet<String>,
    ) -> Result<Self, AuthError> {
        if scopes.is_empty() || scopes.iter().any(|scope| scope.trim().is_empty()) {
            return Err(auth_error(
                AuthErrorCategory::InvalidCredentials,
                false,
                "Google OAuth configuration is invalid",
            ));
        }
        Ok(Self {
            client_id,
            client_secret,
            refresh_token,
            scopes,
        })
    }

    #[must_use]
    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }
}

impl fmt::Debug for GoogleOAuthConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GoogleOAuthConfig(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationCodeRequest {
    code: SecretString,
    redirect_uri: String,
}

impl AuthorizationCodeRequest {
    pub fn new(code: SecretString, redirect_uri: impl Into<String>) -> Result<Self, AuthError> {
        let redirect_uri = redirect_uri.into();
        if redirect_uri.len() > 2_048
            || redirect_uri.contains('#')
            || redirect_uri.chars().any(char::is_control)
            || !(redirect_uri.starts_with("https://")
                || redirect_uri.starts_with("http://127.0.0.1")
                || redirect_uri.starts_with("http://localhost"))
        {
            return Err(auth_error(
                AuthErrorCategory::InvalidCredentials,
                false,
                "Google OAuth redirect URI is invalid",
            ));
        }
        Ok(Self { code, redirect_uri })
    }
}

impl fmt::Debug for AuthorizationCodeRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthorizationCodeRequest(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OfflineTokenGrant {
    access_token: AccessToken,
    refresh_token: SecretString,
    scopes: BTreeSet<String>,
}

impl OfflineTokenGrant {
    #[must_use]
    pub fn access_token(&self) -> &AccessToken {
        &self.access_token
    }

    #[must_use]
    pub fn refresh_token(&self) -> &SecretString {
        &self.refresh_token
    }

    #[must_use]
    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }
}

impl fmt::Debug for OfflineTokenGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OfflineTokenGrant(<redacted>)")
    }
}

pub struct GoogleOAuthHttpEndpoint<T, C> {
    transport: T,
    clock: C,
    policy: GoogleHttpPolicy,
    config: GoogleOAuthConfig,
}

impl<T, C> GoogleOAuthHttpEndpoint<T, C> {
    #[must_use]
    pub fn new(
        transport: T,
        clock: C,
        policy: GoogleHttpPolicy,
        config: GoogleOAuthConfig,
    ) -> Self {
        Self {
            transport,
            clock,
            policy,
            config,
        }
    }
}

impl<T, C> fmt::Debug for GoogleOAuthHttpEndpoint<T, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GoogleOAuthHttpEndpoint(<redacted>)")
    }
}

impl<T: GoogleHttpTransport, C: GoogleClock> GoogleOAuthHttpEndpoint<T, C> {
    pub fn exchange_authorization_code(
        &mut self,
        request: AuthorizationCodeRequest,
    ) -> Result<OfflineTokenGrant, AuthError> {
        let body = form_body(&[
            ("code", request.code.expose_secret()),
            ("client_id", self.config.client_id.expose_secret()),
            ("client_secret", self.config.client_secret.expose_secret()),
            ("redirect_uri", &request.redirect_uri),
            ("grant_type", "authorization_code"),
        ]);
        let now = self.clock.now();
        let value = self.token_request(body)?;
        let access_token = parse_access_token(&value, now)?;
        let refresh_token = json_string(&value, "refresh_token")
            .and_then(|value| SecretString::from_raw("google-refresh-token", value).ok())
            .ok_or_else(|| {
                auth_error(
                    AuthErrorCategory::RefreshUnavailable,
                    false,
                    "Google offline authorization returned no refresh token",
                )
            })?;
        let scopes = value
            .get("scope")
            .and_then(Value::as_str)
            .map(|value| {
                value
                    .split_ascii_whitespace()
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_else(|| self.config.scopes.clone());
        self.config.refresh_token = Some(refresh_token.clone());
        Ok(OfflineTokenGrant {
            access_token,
            refresh_token,
            scopes,
        })
    }

    fn refresh_at(&self, now: SystemTime) -> Result<AccessToken, AuthError> {
        let refresh_token = self.config.refresh_token.as_ref().ok_or_else(|| {
            auth_error(
                AuthErrorCategory::RefreshUnavailable,
                false,
                "Google refresh token is not configured",
            )
        })?;
        let body = form_body(&[
            ("client_id", self.config.client_id.expose_secret()),
            ("client_secret", self.config.client_secret.expose_secret()),
            ("refresh_token", refresh_token.expose_secret()),
            ("grant_type", "refresh_token"),
        ]);
        parse_access_token(&self.token_request(body)?, now)
    }

    fn token_request(&self, body: Vec<u8>) -> Result<Value, AuthError> {
        let request = GoogleHttpRequest::new(GoogleHttpMethod::Post, TOKEN_URL.to_owned())
            .header("Content-Type", "application/x-www-form-urlencoded".to_owned())
            .body(body);
        let response = self
            .transport
            .execute(&request, self.policy.max_json_bytes)
            .map_err(map_oauth_transport)?;
        if response.body().len() > self.policy.max_json_bytes {
            return Err(auth_error(
                AuthErrorCategory::ProviderUnavailable,
                true,
                "Google OAuth response exceeded the configured limit",
            ));
        }
        match response.status() {
            200..=299 => serde_json::from_slice(response.body()).map_err(|_| {
                auth_error(
                    AuthErrorCategory::ProviderUnavailable,
                    false,
                    "Google OAuth response was malformed",
                )
            }),
            300..=399 => Err(auth_error(
                AuthErrorCategory::ProviderUnavailable,
                false,
                "Google OAuth redirect was refused",
            )),
            400 => Err(auth_error(
                AuthErrorCategory::InvalidCredentials,
                false,
                "Google OAuth request was rejected",
            )),
            401 => Err(auth_error(
                AuthErrorCategory::Revoked,
                false,
                "Google OAuth authorization is invalid or revoked",
            )),
            403 => Err(auth_error(
                AuthErrorCategory::InsufficientScope,
                false,
                "Google OAuth authorization is forbidden",
            )),
            429 | 500..=599 => Err(auth_error(
                AuthErrorCategory::ProviderUnavailable,
                true,
                "Google OAuth provider is temporarily unavailable",
            )),
            _ => Err(auth_error(
                AuthErrorCategory::ProviderUnavailable,
                false,
                "Google OAuth provider returned an unexpected response",
            )),
        }
    }
}

impl<T: GoogleHttpTransport, C: GoogleClock> TokenEndpoint for GoogleOAuthHttpEndpoint<T, C> {
    fn refresh(&mut self, request: TokenRefreshRequest<'_>) -> Result<AccessToken, AuthError> {
        if request.token_uri() != TOKEN_URL {
            return Err(auth_error(
                AuthErrorCategory::InvalidCredentials,
                false,
                "Google OAuth token endpoint is invalid",
            ));
        }
        self.refresh_at(self.clock.now())
    }
}

fn parse_access_token(value: &Value, now: SystemTime) -> Result<AccessToken, AuthError> {
    let raw = json_string(value, "access_token").ok_or_else(malformed_oauth)?;
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_u64)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(malformed_oauth)?;
    let expires_at = now
        .checked_add(Duration::from_secs(expires_in))
        .ok_or_else(malformed_oauth)?;
    let token = AccessToken::new(
        SecretString::from_raw("google-access-token", raw).map_err(|_| malformed_oauth())?,
        expires_at,
    );
    if !token.is_usable_at(now) {
        return Err(auth_error(
            AuthErrorCategory::RefreshUnavailable,
            true,
            "Google OAuth token lifetime was too short",
        ));
    }
    Ok(token)
}

fn malformed_oauth() -> AuthError {
    auth_error(
        AuthErrorCategory::ProviderUnavailable,
        false,
        "Google OAuth response was malformed",
    )
}

pub trait GoogleAccessTokenProvider {
    fn access_token_at(&mut self, now: SystemTime) -> Result<AccessToken, AuthError>;
}

impl<T: TokenEndpoint> GoogleAccessTokenProvider for GoogleAuthClient<T> {
    fn access_token_at(&mut self, now: SystemTime) -> Result<AccessToken, AuthError> {
        self.access_token(now).cloned()
    }
}

pub struct GoogleDriveHttpClient<A, T, C> {
    auth: RefCell<A>,
    transport: T,
    clock: C,
    policy: GoogleHttpPolicy,
    scopes: BTreeSet<String>,
}

impl<A, T, C> GoogleDriveHttpClient<A, T, C> {
    #[must_use]
    pub fn new(
        auth: A,
        transport: T,
        clock: C,
        policy: GoogleHttpPolicy,
        scopes: BTreeSet<String>,
    ) -> Self {
        Self {
            auth: RefCell::new(auth),
            transport,
            clock,
            policy,
            scopes,
        }
    }
}

impl<A: GoogleAccessTokenProvider, T: GoogleHttpTransport, C: GoogleClock>
    GoogleDriveHttpClient<A, T, C>
{
    fn execute(
        &self,
        mut request: GoogleHttpRequest,
        limit: usize,
        operation: &'static str,
    ) -> Result<GoogleHttpResponse, ProviderError> {
        let token = self
            .auth
            .borrow_mut()
            .access_token_at(self.clock.now())
            .map_err(map_auth_provider)?;
        request = request.header(
            "Authorization",
            format!("Bearer {}", token.expose_for_authorization()),
        );
        let response = self
            .transport
            .execute(&request, limit)
            .map_err(|error| map_drive_transport(operation, error))?;
        if response.body().len() > limit {
            return Err(provider_error(
                operation,
                ProviderErrorCategory::Internal,
                "Google response exceeded the configured limit",
            ));
        }
        Ok(response)
    }

    fn json(
        &self,
        request: GoogleHttpRequest,
        operation: &'static str,
    ) -> Result<Value, ProviderError> {
        let response = self.execute(request, self.policy.max_json_bytes, operation)?;
        drive_status(operation, &response)?;
        serde_json::from_slice(response.body()).map_err(|_| {
            provider_error(
                operation,
                ProviderErrorCategory::Internal,
                "Google JSON response was malformed",
            )
        })
    }

    fn metadata(&self, file_id: &str) -> Result<DriveMetadata, ProviderError> {
        validate_id(file_id, "get_metadata")?;
        let url = format!(
            "{FILES_URL}/{}?fields={}",
            encode(file_id),
            encode(METADATA_FIELDS)
        );
        parse_metadata(
            &self.json(
                GoogleHttpRequest::new(GoogleHttpMethod::Get, url),
                "get_metadata",
            )?,
            "get_metadata",
        )
    }

    fn outcome(
        &self,
        value: Value,
        operation: &'static str,
    ) -> Result<DriveMutationOutcome, ProviderError> {
        let id = json_string(&value, "id").ok_or_else(|| malformed_drive(operation))?;
        Ok(DriveMutationOutcome {
            provider_id: id.to_owned(),
            revision_token: value
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })
    }
}

impl<A: GoogleAccessTokenProvider, T: GoogleHttpTransport, C: GoogleClock> GoogleProviderClient
    for GoogleDriveHttpClient<A, T, C>
{
    fn observed_scopes(&self) -> Result<BTreeSet<String>, AuthError> {
        Ok(self.scopes.clone())
    }

    fn root_is_accessible(&self, root_id: &str) -> Result<bool, AuthError> {
        match self.metadata(root_id) {
            Ok(metadata) => Ok(metadata.kind == DriveEntryKind::Folder),
            Err(error) if error.category() == ProviderErrorCategory::NotFound => Ok(false),
            Err(error) => Err(map_provider_auth(error)),
        }
    }
}

impl<A: GoogleAccessTokenProvider, T: GoogleHttpTransport, C: GoogleClock> DriveProvider
    for GoogleDriveHttpClient<A, T, C>
{
    fn list_children(&self, folder_id: &str) -> Result<Vec<DriveMetadata>, ProviderError> {
        validate_id(folder_id, "list_children")?;
        let query = format!("'{folder_id}' in parents and trashed = false");
        let fields = format!("nextPageToken,files({METADATA_FIELDS})");
        let mut token = None;
        let mut seen = HashSet::new();
        let mut files = Vec::new();
        for _ in 0..self.policy.max_pages {
            let mut url = format!(
                "{FILES_URL}?q={}&pageSize=1000&fields={}",
                encode(&query),
                encode(&fields)
            );
            if let Some(page_token) = &token {
                url.push_str("&pageToken=");
                url.push_str(&encode(page_token));
            }
            let value = self.json(
                GoogleHttpRequest::new(GoogleHttpMethod::Get, url),
                "list_children",
            )?;
            let object = value.as_object().ok_or_else(|| malformed_drive("list_children"))?;
            let page = object
                .get("files")
                .and_then(Value::as_array)
                .ok_or_else(|| malformed_drive("list_children"))?;
            for file in page {
                if files.len() >= self.policy.max_items {
                    return Err(provider_error(
                        "list_children",
                        ProviderErrorCategory::InvalidRequest,
                        "Google file listing exceeded the configured item limit",
                    ));
                }
                files.push(parse_metadata(file, "list_children")?);
            }
            token = optional_string(object, "nextPageToken")?;
            match &token {
                None => return Ok(files),
                Some(value) if seen.insert(value.clone()) => {}
                Some(_) => {
                    return Err(provider_error(
                        "list_children",
                        ProviderErrorCategory::Internal,
                        "Google file pagination loop was detected",
                    ));
                }
            }
        }
        Err(provider_error(
            "list_children",
            ProviderErrorCategory::Internal,
            "Google file listing exceeded the configured page limit",
        ))
    }

    fn get_metadata(&self, file_id: &str) -> Result<DriveMetadata, ProviderError> {
        self.metadata(file_id)
    }

    fn download_file(&self, file_id: &str) -> Result<Vec<u8>, ProviderError> {
        let metadata = self.metadata(file_id)?;
        if matches!(
            classify_drive_metadata(&metadata),
            DriveEntryClassification::Unsupported(_)
        ) {
            return Err(ProviderError::unsupported("download_file"));
        }
        let response = self.execute(
            GoogleHttpRequest::new(
                GoogleHttpMethod::Get,
                format!("{FILES_URL}/{}?alt=media", encode(file_id)),
            ),
            self.policy.max_download_bytes,
            "download_file",
        )?;
        drive_status("download_file", &response)?;
        Ok(response.body().to_vec())
    }

    fn upload_file(
        &mut self,
        request: DriveUploadRequest,
    ) -> Result<DriveMutationOutcome, ProviderError> {
        validate_id(&request.parent_id, "upload_file")?;
        if request.name.trim().is_empty() || request.name.len() > 1_024 {
            return Err(provider_error(
                "upload_file",
                ProviderErrorCategory::InvalidRequest,
                "Google upload name is invalid",
            ));
        }
        validate_upload(&request.mime_type, &request.content, self.policy)?;
        let metadata = serde_json::to_vec(&serde_json::json!({
            "name": &request.name,
            "parents": [&request.parent_id],
            "mimeType": &request.mime_type
        }))
        .map_err(|_| malformed_drive("upload_file"))?;
        let body = multipart(&metadata, &request.content, &request.mime_type)?;
        let url = format!(
            "{UPLOAD_URL}?uploadType=multipart&fields={}",
            encode("id,version")
        );
        self.outcome(
            self.json(
                GoogleHttpRequest::new(GoogleHttpMethod::Post, url)
                    .header(
                        "Content-Type",
                        format!("multipart/related; boundary={MULTIPART_BOUNDARY}"),
                    )
                    .body(body),
                "upload_file",
            )?,
            "upload_file",
        )
    }

    fn update_file(
        &mut self,
        request: DriveUpdateRequest,
    ) -> Result<DriveMutationOutcome, ProviderError> {
        validate_id(&request.file_id, "update_file")?;
        validate_upload(&request.mime_type, &request.content, self.policy)?;
        let url = format!(
            "{UPLOAD_URL}/{}?uploadType=media&fields={}",
            encode(&request.file_id),
            encode("id,version")
        );
        self.outcome(
            self.json(
                GoogleHttpRequest::new(GoogleHttpMethod::Patch, url)
                    .header("Content-Type", request.mime_type)
                    .body(request.content),
                "update_file",
            )?,
            "update_file",
        )
    }

    fn trash_file(&mut self, file_id: &str) -> Result<DriveMutationOutcome, ProviderError> {
        validate_id(file_id, "trash_file")?;
        let url = format!(
            "{FILES_URL}/{}?fields={}",
            encode(file_id),
            encode("id,version")
        );
        self.outcome(
            self.json(
                GoogleHttpRequest::new(GoogleHttpMethod::Patch, url)
                    .header("Content-Type", "application/json".to_owned())
                    .body(br#"{"trashed":true}"#.to_vec()),
                "trash_file",
            )?,
            "trash_file",
        )
    }

    fn delete_file(&mut self, _file_id: &str) -> Result<DriveMutationOutcome, ProviderError> {
        Err(ProviderError::unsupported("delete_file"))
    }
}

impl<A: GoogleAccessTokenProvider, T: GoogleHttpTransport, C: GoogleClock>
    DriveChangeFeedProvider for GoogleDriveHttpClient<A, T, C>
{
    fn get_start_page_token(&self) -> Result<String, ProviderError> {
        let value = self.json(
            GoogleHttpRequest::new(
                GoogleHttpMethod::Get,
                format!("{START_TOKEN_URL}?fields={}", encode("startPageToken")),
            ),
            "get_start_page_token",
        )?;
        json_string(&value, "startPageToken")
            .map(str::to_owned)
            .ok_or_else(|| malformed_drive("get_start_page_token"))
    }

    fn list_changes(&self, page_token: &str) -> Result<DriveChangePoll, ProviderError> {
        validate_token(page_token)?;
        let fields = format!(
            "nextPageToken,newStartPageToken,changes(fileId,removed,file({METADATA_FIELDS}))"
        );
        let url = format!(
            "{CHANGES_URL}?pageToken={}&pageSize=1000&includeRemoved=true&restrictToMyDrive=true&fields={}",
            encode(page_token),
            encode(&fields)
        );
        let response = self.execute(
            GoogleHttpRequest::new(GoogleHttpMethod::Get, url),
            self.policy.max_json_bytes,
            "list_changes",
        )?;
        if response.status() == 410 {
            return DriveChangePoll::cursor_invalidated(self.get_start_page_token()?)
                .map_err(|_| malformed_drive("list_changes"));
        }
        drive_status("list_changes", &response)?;
        let value: Value = serde_json::from_slice(response.body())
            .map_err(|_| malformed_drive("list_changes"))?;
        let object = value
            .as_object()
            .ok_or_else(|| malformed_drive("list_changes"))?;
        let raw_changes = object
            .get("changes")
            .and_then(Value::as_array)
            .ok_or_else(|| malformed_drive("list_changes"))?;
        if raw_changes.len() > self.policy.max_items {
            return Err(provider_error(
                "list_changes",
                ProviderErrorCategory::InvalidRequest,
                "Google changes exceeded the configured item limit",
            ));
        }
        let changes = raw_changes
            .iter()
            .map(parse_change)
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(next) = optional_string(object, "nextPageToken")? {
            return DriveChangePage::intermediate(changes, next)
                .map(DriveChangePoll::Page)
                .map_err(|_| malformed_drive("list_changes"));
        }
        let next = json_string(&value, "newStartPageToken")
            .ok_or_else(|| malformed_drive("list_changes"))?;
        DriveChangePage::final_page(changes, next)
            .map(DriveChangePoll::Page)
            .map_err(|_| malformed_drive("list_changes"))
    }
}

fn parse_change(value: &Value) -> Result<DriveChangeEntry, ProviderError> {
    let object = value
        .as_object()
        .ok_or_else(|| malformed_drive("list_changes"))?;
    let id = json_string(value, "fileId").ok_or_else(|| malformed_drive("list_changes"))?;
    if object.get("removed").and_then(Value::as_bool) == Some(true) {
        return DriveChangeEntry::removed(id).map_err(|_| malformed_drive("list_changes"));
    }
    let Some(file) = object.get("file") else {
        return DriveChangeEntry::metadata_missing(id)
            .map_err(|_| malformed_drive("list_changes"));
    };
    if file.get("trashed").and_then(Value::as_bool) == Some(true) {
        return DriveChangeEntry::removed(id).map_err(|_| malformed_drive("list_changes"));
    }
    let metadata = parse_metadata(file, "list_changes")?;
    if metadata.id != id {
        return Err(malformed_drive("list_changes"));
    }
    let entry = DriveChangeEntry::file(metadata).map_err(|_| malformed_drive("list_changes"))?;
    match file.get("version").and_then(Value::as_str) {
        Some(version) => entry
            .with_drive_version(version)
            .map_err(|_| malformed_drive("list_changes")),
        None => Ok(entry),
    }
}

fn parse_metadata(value: &Value, operation: &'static str) -> Result<DriveMetadata, ProviderError> {
    let object = value
        .as_object()
        .ok_or_else(|| malformed_drive(operation))?;
    let id = json_string(value, "id").ok_or_else(|| malformed_drive(operation))?;
    let name = json_string(value, "name").ok_or_else(|| malformed_drive(operation))?;
    let mime_type = object
        .get("mimeType")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let kind = if object.get("driveId").and_then(Value::as_str).is_some() {
        DriveEntryKind::SharedDrive
    } else {
        DriveEntryKind::from_mime_type(mime_type.as_deref())
    };
    let parent_ids = object
        .get("parents")
        .and_then(Value::as_array)
        .map(|parents| {
            parents
                .iter()
                .map(|parent| {
                    parent
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| malformed_drive(operation))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(DriveMetadata {
        id: id.to_owned(),
        name: name.to_owned(),
        mime_type,
        kind,
        parent_ids,
        modified_time: object
            .get("modifiedTime")
            .and_then(Value::as_str)
            .map(str::to_owned),
        size_bytes: object.get("size").and_then(json_u64),
        md5_checksum: object
            .get("md5Checksum")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn drive_status(
    operation: &'static str,
    response: &GoogleHttpResponse,
) -> Result<(), ProviderError> {
    match response.status() {
        200..=299 => Ok(()),
        300..=399 => Err(provider_error(
            operation,
            ProviderErrorCategory::InvalidRequest,
            "Google redirect was refused",
        )),
        401 | 403 => Err(provider_error(
            operation,
            ProviderErrorCategory::Auth,
            "Google authorization was rejected",
        )),
        404 => Err(ProviderError::not_found(operation)),
        429 => Err(provider_error(
            operation,
            ProviderErrorCategory::RateLimit,
            "Google rate limit was reached",
        )),
        500..=599 => Err(provider_error(
            operation,
            ProviderErrorCategory::ProviderUnavailable,
            "Google provider is temporarily unavailable",
        )),
        _ => Err(provider_error(
            operation,
            ProviderErrorCategory::InvalidRequest,
            "Google provider rejected the request",
        )),
    }
}

fn validate_upload(
    mime_type: &str,
    content: &[u8],
    policy: GoogleHttpPolicy,
) -> Result<(), ProviderError> {
    if mime_type.trim().is_empty() || content.len() > policy.max_upload_bytes {
        return Err(provider_error(
            "validate_upload",
            ProviderErrorCategory::InvalidRequest,
            "Google upload request is invalid or too large",
        ));
    }
    if matches!(
        mime_type,
        MIME_GOOGLE_FOLDER
            | MIME_GOOGLE_DOC
            | MIME_GOOGLE_SHEET
            | MIME_GOOGLE_SLIDE
            | MIME_GOOGLE_SHORTCUT
    ) {
        return Err(ProviderError::unsupported("validate_upload"));
    }
    Ok(())
}

fn multipart(metadata: &[u8], content: &[u8], mime_type: &str) -> Result<Vec<u8>, ProviderError> {
    let marker = MULTIPART_BOUNDARY.as_bytes();
    if metadata.windows(marker.len()).any(|window| window == marker)
        || content.windows(marker.len()).any(|window| window == marker)
    {
        return Err(provider_error(
            "upload_file",
            ProviderErrorCategory::InvalidRequest,
            "Google upload conflicts with the multipart boundary",
        ));
    }
    let mut body = Vec::with_capacity(metadata.len() + content.len() + 256);
    body.extend_from_slice(format!("--{MULTIPART_BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata);
    body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(format!("Content-Type: {mime_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    Ok(body)
}

fn validate_id(value: &str, operation: &'static str) -> Result<(), ProviderError> {
    if value.is_empty()
        || value.len() > 512
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(provider_error(
            operation,
            ProviderErrorCategory::InvalidRequest,
            "Google provider identifier is invalid",
        ));
    }
    Ok(())
}

fn validate_token(value: &str) -> Result<(), ProviderError> {
    if value.trim().is_empty() || value.len() > 8 * 1024 || value.chars().any(char::is_control) {
        return Err(provider_error(
            "list_changes",
            ProviderErrorCategory::InvalidRequest,
            "Google change token is invalid",
        ));
    }
    Ok(())
}

fn json_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && value.len() <= 16 * 1024)
}

fn optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, ProviderError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() && value.len() <= 16 * 1024 => {
            Ok(Some(value.clone()))
        }
        _ => Err(malformed_drive("decode_pagination")),
    }
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn form_body(values: &[(&str, &str)]) -> Vec<u8> {
    values
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
        .into_bytes()
}

fn encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(hex(byte >> 4));
            output.push(hex(byte & 0x0f));
        }
    }
    output
}

fn hex(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'A' + value - 10),
        _ => unreachable!("hex nibble is bounded"),
    }
}

fn map_oauth_transport(error: GoogleHttpTransportError) -> AuthError {
    auth_error(
        AuthErrorCategory::ProviderUnavailable,
        true,
        match error {
            GoogleHttpTransportError::Timeout => "Google OAuth request timed out",
            GoogleHttpTransportError::Unavailable => "Google OAuth transport is unavailable",
        },
    )
}

fn map_drive_transport(
    operation: &'static str,
    error: GoogleHttpTransportError,
) -> ProviderError {
    provider_error(
        operation,
        ProviderErrorCategory::ProviderUnavailable,
        match error {
            GoogleHttpTransportError::Timeout => "Google request timed out",
            GoogleHttpTransportError::Unavailable => "Google transport is unavailable",
        },
    )
}

fn map_auth_provider(error: AuthError) -> ProviderError {
    let category = match error.category() {
        AuthErrorCategory::NotConfigured
        | AuthErrorCategory::InvalidCredentials
        | AuthErrorCategory::Revoked
        | AuthErrorCategory::InsufficientScope => ProviderErrorCategory::Auth,
        AuthErrorCategory::RefreshUnavailable | AuthErrorCategory::ProviderUnavailable => {
            ProviderErrorCategory::ProviderUnavailable
        }
    };
    provider_error(
        "authorize_google_request",
        category,
        "Google access token is unavailable",
    )
}

fn map_provider_auth(error: ProviderError) -> AuthError {
    match error.category() {
        ProviderErrorCategory::Auth => auth_error(
            AuthErrorCategory::Revoked,
            false,
            "Google authorization is revoked or inaccessible",
        ),
        ProviderErrorCategory::RateLimit
        | ProviderErrorCategory::ProviderUnavailable
        | ProviderErrorCategory::Internal => auth_error(
            AuthErrorCategory::ProviderUnavailable,
            true,
            "Google provider is unavailable",
        ),
        ProviderErrorCategory::NotFound
        | ProviderErrorCategory::Unsupported
        | ProviderErrorCategory::InvalidRequest => auth_error(
            AuthErrorCategory::InvalidCredentials,
            false,
            "Google Drive root configuration is invalid",
        ),
    }
}

fn malformed_drive(operation: &'static str) -> ProviderError {
    provider_error(
        operation,
        ProviderErrorCategory::Internal,
        "Google response was malformed",
    )
}

const fn auth_error(
    category: AuthErrorCategory,
    retryable: bool,
    message: &'static str,
) -> AuthError {
    AuthError::new(category, retryable, message)
}

const fn provider_error(
    operation: &'static str,
    category: ProviderErrorCategory,
    message: &'static str,
) -> ProviderError {
    ProviderError::new(operation, category, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::GoogleCredentials;
    use crate::drive::UnsupportedEntryReason;
    use std::collections::VecDeque;

    const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
    const NOW: SystemTime = SystemTime::UNIX_EPOCH;

    #[derive(Clone, Copy)]
    struct Clock;

    impl GoogleClock for Clock {
        fn now(&self) -> SystemTime {
            NOW
        }
    }

    #[derive(Default)]
    struct FakeHttp {
        responses: RefCell<VecDeque<Result<GoogleHttpResponse, GoogleHttpTransportError>>>,
        requests: RefCell<Vec<GoogleHttpRequest>>,
    }

    impl FakeHttp {
        fn new(
            responses: impl IntoIterator<
                Item = Result<GoogleHttpResponse, GoogleHttpTransportError>,
            >,
        ) -> Self {
            Self {
                responses: RefCell::new(responses.into_iter().collect()),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl GoogleHttpTransport for FakeHttp {
        fn execute(
            &self,
            request: &GoogleHttpRequest,
            _max_response_bytes: usize,
        ) -> Result<GoogleHttpResponse, GoogleHttpTransportError> {
            self.requests.borrow_mut().push(request.clone());
            self.responses
                .borrow_mut()
                .pop_front()
                .expect("configured response")
        }
    }

    #[derive(Clone)]
    struct Token(AccessToken);

    impl GoogleAccessTokenProvider for Token {
        fn access_token_at(&mut self, _now: SystemTime) -> Result<AccessToken, AuthError> {
            Ok(self.0.clone())
        }
    }

    fn secret(key: &'static str, value: &str) -> SecretString {
        SecretString::from_raw(key, value).expect("secret")
    }

    fn access_token() -> AccessToken {
        AccessToken::new(
            secret("access", "synthetic-access"),
            NOW + Duration::from_secs(3_600),
        )
    }

    fn oauth(refresh: Option<&str>) -> GoogleOAuthConfig {
        GoogleOAuthConfig::new(
            secret("client", "synthetic-client"),
            secret("client-secret", "synthetic-secret"),
            refresh.map(|value| secret("refresh", value)),
            BTreeSet::from([DRIVE_SCOPE.to_owned()]),
        )
        .expect("oauth config")
    }

    fn drive(
        responses: impl IntoIterator<Item = Result<GoogleHttpResponse, GoogleHttpTransportError>>,
    ) -> GoogleDriveHttpClient<Token, FakeHttp, Clock> {
        GoogleDriveHttpClient::new(
            Token(access_token()),
            FakeHttp::new(responses),
            Clock,
            GoogleHttpPolicy::default(),
            BTreeSet::from([DRIVE_SCOPE.to_owned()]),
        )
    }

    #[test]
    fn authorization_code_and_refresh_are_fakeable_deterministic_and_redacted() {
        let transport = FakeHttp::new([Ok(GoogleHttpResponse::new(
            200,
            br#"{"access_token":"grant-access","expires_in":3600,"refresh_token":"grant-refresh","scope":"https://www.googleapis.com/auth/drive"}"#,
        ))]);
        let mut endpoint = GoogleOAuthHttpEndpoint::new(
            transport,
            Clock,
            GoogleHttpPolicy::default(),
            oauth(None),
        );
        let grant = endpoint
            .exchange_authorization_code(
                AuthorizationCodeRequest::new(
                    secret("code", "a code+value"),
                    "https://localhost/callback",
                )
                .expect("request"),
            )
            .expect("grant");
        assert!(grant.access_token().is_usable_at(NOW));
        assert!(grant.scopes().contains(DRIVE_SCOPE));
        let request = &endpoint.transport.requests.borrow()[0];
        assert_eq!(request.url_for_transport(), TOKEN_URL);
        let body = std::str::from_utf8(request.body_for_transport()).expect("form");
        assert!(body.contains("code=a%20code%2Bvalue"));
        assert!(!format!("{endpoint:?} {grant:?} {request:?}").contains("synthetic"));

        let refresh_transport = FakeHttp::new([Ok(GoogleHttpResponse::new(
            200,
            br#"{"access_token":"refreshed","expires_in":300}"#,
        ))]);
        let refresh_endpoint = GoogleOAuthHttpEndpoint::new(
            refresh_transport,
            Clock,
            GoogleHttpPolicy::default(),
            oauth(Some("synthetic-refresh")),
        );
        let credentials = GoogleCredentials::parse(&format!(
            "haze-gdrive-credential-v1\nclient_id=synthetic-client\nclient_secret=synthetic-secret\nrefresh_token=synthetic-refresh\ntoken_uri={TOKEN_URL}\nscopes={DRIVE_SCOPE}\n"
        ))
        .expect("credentials");
        let mut client = GoogleAuthClient::new(credentials, refresh_endpoint);
        assert_eq!(
            client
                .access_token(NOW)
                .expect("refreshed")
                .expose_for_authorization(),
            "refreshed"
        );
    }

    #[test]
    fn files_list_download_create_update_and_trash_use_existing_trait() {
        let mut client = drive([
            Ok(GoogleHttpResponse::new(
                200,
                br#"{"files":[{"id":"a","name":"a.md","mimeType":"text/markdown"}],"nextPageToken":"p2"}"#,
            )),
            Ok(GoogleHttpResponse::new(
                200,
                br#"{"files":[{"id":"doc","name":"Doc","mimeType":"application/vnd.google-apps.document"}]}"#,
            )),
            Ok(GoogleHttpResponse::new(
                200,
                br#"{"id":"a","name":"a.md","mimeType":"text/markdown"}"#,
            )),
            Ok(GoogleHttpResponse::new(200, b"hello")),
            Ok(GoogleHttpResponse::new(200, br#"{"id":"new","version":"1"}"#)),
            Ok(GoogleHttpResponse::new(200, br#"{"id":"new","version":"2"}"#)),
            Ok(GoogleHttpResponse::new(200, br#"{"id":"new","version":"3"}"#)),
        ]);
        let files = client.list_children("root").expect("list");
        assert_eq!(files.len(), 2);
        assert_eq!(
            classify_drive_metadata(&files[1]),
            DriveEntryClassification::Unsupported(
                UnsupportedEntryReason::GoogleWorkspaceDocument
            )
        );
        assert_eq!(client.download_file("a").expect("download"), b"hello");
        assert_eq!(
            client
                .upload_file(DriveUploadRequest {
                    parent_id: "root".to_owned(),
                    name: "new.md".to_owned(),
                    mime_type: "text/markdown".to_owned(),
                    content: b"new".to_vec(),
                })
                .expect("create")
                .provider_id,
            "new"
        );
        assert_eq!(
            client
                .update_file(DriveUpdateRequest {
                    file_id: "new".to_owned(),
                    mime_type: "text/markdown".to_owned(),
                    content: b"updated".to_vec(),
                })
                .expect("update")
                .revision_token
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            client
                .trash_file("new")
                .expect("trash")
                .revision_token
                .as_deref(),
            Some("3")
        );
    }

    #[test]
    fn changes_pages_and_cursor_invalidation_use_existing_trait() {
        let client = drive([
            Ok(GoogleHttpResponse::new(200, br#"{"startPageToken":"s1"}"#)),
            Ok(GoogleHttpResponse::new(
                200,
                br#"{"changes":[{"fileId":"a","file":{"id":"a","name":"a.md","mimeType":"text/markdown","version":"7"}},{"fileId":"gone","removed":true}],"nextPageToken":"p2"}"#,
            )),
            Ok(GoogleHttpResponse::new(410, b"raw-provider-body")),
            Ok(GoogleHttpResponse::new(
                200,
                br#"{"startPageToken":"replacement"}"#,
            )),
        ]);
        assert_eq!(client.get_start_page_token().expect("start"), "s1");
        match client.list_changes("s1").expect("page") {
            DriveChangePoll::Page(page) => {
                assert_eq!(page.changes().len(), 2);
                assert_eq!(page.changes()[0].drive_version(), Some("7"));
                assert!(page.changes()[1].is_removed());
            }
            DriveChangePoll::CursorInvalidated { .. } => panic!("unexpected invalidation"),
        }
        assert!(matches!(
            client.list_changes("stale").expect("replacement"),
            DriveChangePoll::CursorInvalidated { .. }
        ));
    }

    #[test]
    fn limits_redirects_malformed_errors_and_unsupported_entries_fail_safely() {
        let timeout = drive([Err(GoogleHttpTransportError::Timeout)]);
        assert_eq!(
            timeout.get_metadata("a").expect_err("timeout").category(),
            ProviderErrorCategory::ProviderUnavailable
        );
        for (status, category) in [
            (302, ProviderErrorCategory::InvalidRequest),
            (401, ProviderErrorCategory::Auth),
            (403, ProviderErrorCategory::Auth),
            (404, ProviderErrorCategory::NotFound),
            (429, ProviderErrorCategory::RateLimit),
            (500, ProviderErrorCategory::ProviderUnavailable),
        ] {
            let client = drive([Ok(GoogleHttpResponse::new(
                status,
                b"synthetic-access raw-provider-body",
            ))]);
            let error = client.get_metadata("a").expect_err("status");
            assert_eq!(error.category(), category);
            let rendered = format!("{error:?} {error}");
            assert!(!rendered.contains("synthetic-access"));
            assert!(!rendered.contains("raw-provider-body"));
        }
        let malformed = drive([Ok(GoogleHttpResponse::new(200, b"not-json"))]);
        assert_eq!(
            malformed
                .get_metadata("a")
                .expect_err("malformed")
                .category(),
            ProviderErrorCategory::Internal
        );
        let policy = GoogleHttpPolicy {
            max_json_bytes: 4,
            ..GoogleHttpPolicy::default()
        };
        let oversized = GoogleDriveHttpClient::new(
            Token(access_token()),
            FakeHttp::new([Ok(GoogleHttpResponse::new(200, b"12345"))]),
            Clock,
            policy,
            BTreeSet::from([DRIVE_SCOPE.to_owned()]),
        );
        assert_eq!(
            oversized
                .get_metadata("a")
                .expect_err("oversized")
                .category(),
            ProviderErrorCategory::Internal
        );
        let mut unsupported = drive([]);
        assert_eq!(
            unsupported
                .upload_file(DriveUploadRequest {
                    parent_id: "root".to_owned(),
                    name: "Doc".to_owned(),
                    mime_type: MIME_GOOGLE_DOC.to_owned(),
                    content: b"content".to_vec(),
                })
                .expect_err("unsupported")
                .category(),
            ProviderErrorCategory::Unsupported
        );
    }
}
