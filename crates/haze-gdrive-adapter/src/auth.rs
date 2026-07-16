//! Read-only Google credential loading and fake-first authentication boundaries.
//!
//! The credential file is operator-managed and is never rewritten by this crate.

use crate::config::{SecretPath, SecretString};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::time::{Duration, SystemTime};

const FILE_HEADER: &str = "haze-gdrive-credential-v1";
const MAX_CREDENTIAL_FILE_BYTES: usize = 32 * 1024;
const MAX_FIELD_BYTES: usize = 8 * 1024;
const MIN_TOKEN_LIFETIME: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthErrorCategory {
    NotConfigured,
    InvalidCredentials,
    Revoked,
    InsufficientScope,
    RefreshUnavailable,
    ProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthError {
    category: AuthErrorCategory,
    retryable: bool,
    message: &'static str,
}

impl AuthError {
    pub const fn new(category: AuthErrorCategory, retryable: bool, message: &'static str) -> Self {
        Self {
            category,
            retryable,
            message,
        }
    }

    pub const fn category(&self) -> AuthErrorCategory {
        self.category
    }

    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for AuthError {}

#[derive(Clone, PartialEq, Eq)]
pub struct GoogleCredentials {
    client_id: SecretString,
    client_secret: SecretString,
    refresh_token: SecretString,
    token_uri: String,
    scopes: BTreeSet<String>,
}

impl GoogleCredentials {
    pub fn parse(input: &str) -> Result<Self, AuthError> {
        if input.len() > MAX_CREDENTIAL_FILE_BYTES {
            return Err(invalid_credentials());
        }

        let mut lines = input.lines();
        if lines.next() != Some(FILE_HEADER) {
            return Err(invalid_credentials());
        }

        let mut client_id = None;
        let mut client_secret = None;
        let mut refresh_token = None;
        let mut token_uri = None;
        let mut scopes = None;

        for line in lines {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(invalid_credentials)?;
            if value.is_empty() || value.len() > MAX_FIELD_BYTES || value.trim() != value {
                return Err(invalid_credentials());
            }
            match key {
                "client_id" if client_id.is_none() => client_id = Some(secret(value)?),
                "client_secret" if client_secret.is_none() => client_secret = Some(secret(value)?),
                "refresh_token" if refresh_token.is_none() => refresh_token = Some(secret(value)?),
                "token_uri" if token_uri.is_none() => token_uri = Some(parse_token_uri(value)?),
                "scopes" if scopes.is_none() => scopes = Some(parse_scopes(value)?),
                _ => return Err(invalid_credentials()),
            }
        }

        Ok(Self {
            client_id: client_id.ok_or_else(invalid_credentials)?,
            client_secret: client_secret.ok_or_else(invalid_credentials)?,
            refresh_token: refresh_token.ok_or_else(invalid_credentials)?,
            token_uri: token_uri.ok_or_else(invalid_credentials)?,
            scopes: scopes.ok_or_else(invalid_credentials)?,
        })
    }

    pub fn load_read_only(path: &SecretPath) -> Result<Self, AuthError> {
        let bytes = fs::read(path.as_path()).map_err(|_| {
            AuthError::new(
                AuthErrorCategory::NotConfigured,
                false,
                "Google credentials are not configured",
            )
        })?;
        if bytes.len() > MAX_CREDENTIAL_FILE_BYTES {
            return Err(invalid_credentials());
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| invalid_credentials())?;
        Self::parse(text)
    }

    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }

    pub fn token_uri(&self) -> &str {
        &self.token_uri
    }

    fn refresh_request(&self) -> TokenRefreshRequest<'_> {
        TokenRefreshRequest {
            token_uri: &self.token_uri,
            client_id: self.client_id.expose_secret(),
            client_secret: self.client_secret.expose_secret(),
            refresh_token: self.refresh_token.expose_secret(),
        }
    }
}

impl fmt::Debug for GoogleCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GoogleCredentials")
            .field("client_id", &"<redacted-secret>")
            .field("client_secret", &"<redacted-secret>")
            .field("refresh_token", &"<redacted-secret>")
            .field("token_uri", &"<redacted-provider-endpoint>")
            .field("scopes", &"<redacted-authorization-metadata>")
            .finish()
    }
}

impl fmt::Display for GoogleCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GoogleCredentials(<redacted>)")
    }
}

fn secret(value: &str) -> Result<SecretString, AuthError> {
    SecretString::from_raw("google-credential-field", value).map_err(|_| invalid_credentials())
}

fn parse_token_uri(value: &str) -> Result<String, AuthError> {
    if value != "https://oauth2.googleapis.com/token" {
        return Err(invalid_credentials());
    }
    Ok(value.to_owned())
}

fn parse_scopes(value: &str) -> Result<BTreeSet<String>, AuthError> {
    let scopes: BTreeSet<String> = value
        .split(',')
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(str::to_owned)
        .collect();
    if scopes.is_empty() || scopes.iter().any(|scope| scope.len() > 512) {
        return Err(invalid_credentials());
    }
    Ok(scopes)
}

const fn invalid_credentials() -> AuthError {
    AuthError::new(
        AuthErrorCategory::InvalidCredentials,
        false,
        "Google credentials are invalid",
    )
}

const fn refresh_unavailable() -> AuthError {
    AuthError::new(
        AuthErrorCategory::RefreshUnavailable,
        true,
        "Google access-token refresh is unavailable",
    )
}

pub struct TokenRefreshRequest<'a> {
    token_uri: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    refresh_token: &'a str,
}

impl TokenRefreshRequest<'_> {
    pub fn token_uri(&self) -> &str {
        self.token_uri
    }
}

impl fmt::Debug for TokenRefreshRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TokenRefreshRequest(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken {
    value: SecretString,
    expires_at: SystemTime,
}

impl AccessToken {
    pub fn new(value: SecretString, expires_at: SystemTime) -> Self {
        Self { value, expires_at }
    }

    pub fn is_usable_at(&self, now: SystemTime) -> bool {
        self.expires_at
            .duration_since(now)
            .map(|remaining| remaining >= MIN_TOKEN_LIFETIME)
            .unwrap_or(false)
    }

    pub fn expose_for_authorization(&self) -> &str {
        self.value.expose_secret()
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccessToken(<redacted>)")
    }
}

pub trait TokenEndpoint {
    fn refresh(&mut self, request: TokenRefreshRequest<'_>) -> Result<AccessToken, AuthError>;
}

pub struct GoogleAuthClient<T> {
    credentials: GoogleCredentials,
    endpoint: T,
    access_token: Option<AccessToken>,
}

impl<T: TokenEndpoint> GoogleAuthClient<T> {
    pub fn new(credentials: GoogleCredentials, endpoint: T) -> Self {
        Self {
            credentials,
            endpoint,
            access_token: None,
        }
    }

    pub fn access_token(&mut self, now: SystemTime) -> Result<&AccessToken, AuthError> {
        let needs_refresh = self
            .access_token
            .as_ref()
            .map(|token| !token.is_usable_at(now))
            .unwrap_or(true);
        if needs_refresh {
            let request = self.credentials.refresh_request();
            let refreshed = self.endpoint.refresh(request)?;
            if !refreshed.is_usable_at(now) {
                self.access_token = None;
                return Err(refresh_unavailable());
            }
            self.access_token = Some(refreshed);
        }
        self.access_token.as_ref().ok_or_else(refresh_unavailable)
    }
}

pub trait GoogleProviderClient {
    fn observed_scopes(&self) -> Result<BTreeSet<String>, AuthError>;
    fn root_is_accessible(&self, root_id: &str) -> Result<bool, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightReport {
    pub root_accessible: bool,
    pub scopes_satisfied: bool,
}

pub fn preflight(
    provider: &impl GoogleProviderClient,
    required_scopes: &BTreeSet<String>,
    root_id: &str,
) -> Result<PreflightReport, AuthError> {
    if root_id.trim().is_empty() {
        return Err(AuthError::new(
            AuthErrorCategory::NotConfigured,
            false,
            "Google Drive root is not configured",
        ));
    }
    let observed = provider.observed_scopes()?;
    if !required_scopes.is_subset(&observed) {
        return Err(AuthError::new(
            AuthErrorCategory::InsufficientScope,
            false,
            "Google authorization has insufficient scope",
        ));
    }
    if !provider.root_is_accessible(root_id)? {
        return Err(AuthError::new(
            AuthErrorCategory::Revoked,
            false,
            "Google authorization is revoked or inaccessible",
        ));
    }
    Ok(PreflightReport {
        root_accessible: true,
        scopes_satisfied: true,
    })
}

#[derive(Default)]
pub struct FakeTokenEndpoint {
    pub refreshes: usize,
    pub next: Option<Result<AccessToken, AuthError>>,
}

impl TokenEndpoint for FakeTokenEndpoint {
    fn refresh(&mut self, request: TokenRefreshRequest<'_>) -> Result<AccessToken, AuthError> {
        let _ = (
            request.token_uri,
            request.client_id,
            request.client_secret,
            request.refresh_token,
        );
        self.refreshes += 1;
        self.next.take().unwrap_or_else(|| {
            Err(AuthError::new(
                AuthErrorCategory::ProviderUnavailable,
                true,
                "Google provider is unavailable",
            ))
        })
    }
}

#[derive(Default)]
pub struct FakeGoogleProviderClient {
    pub scopes: BTreeSet<String>,
    pub root_accessible: bool,
    pub error: Option<AuthError>,
}

impl GoogleProviderClient for FakeGoogleProviderClient {
    fn observed_scopes(&self) -> Result<BTreeSet<String>, AuthError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(self.scopes.clone())
    }

    fn root_is_accessible(&self, _root_id: &str) -> Result<bool, AuthError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(self.root_accessible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";

    fn valid_credentials() -> String {
        format!(
            "{FILE_HEADER}\nclient_id=synthetic-client\nclient_secret=synthetic-secret\nrefresh_token=synthetic-refresh\ntoken_uri=https://oauth2.googleapis.com/token\nscopes={DRIVE_SCOPE}\n"
        )
    }

    fn fixed_now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000)
    }

    fn token(value: &str, expires_at: SystemTime) -> AccessToken {
        AccessToken::new(
            SecretString::from_raw("test", value).expect("secret"),
            expires_at,
        )
    }

    fn auth_client(endpoint: FakeTokenEndpoint) -> GoogleAuthClient<FakeTokenEndpoint> {
        GoogleAuthClient::new(
            GoogleCredentials::parse(&valid_credentials()).expect("credentials"),
            endpoint,
        )
    }

    #[test]
    fn parses_versioned_credentials_and_redacts_every_surface() {
        let credentials = GoogleCredentials::parse(&valid_credentials()).expect("valid fixture");
        assert!(credentials.scopes().contains(DRIVE_SCOPE));
        let rendered = format!("{credentials:?} {credentials}");
        assert!(!rendered.contains("synthetic"));
        assert!(!rendered.contains("oauth2.googleapis.com"));
        assert!(!rendered.contains(DRIVE_SCOPE));
    }

    #[test]
    fn malformed_or_missing_fields_fail_closed() {
        for fixture in [
            "not-versioned\nclient_id=x",
            "haze-gdrive-credential-v1\nclient_id=x",
            "haze-gdrive-credential-v1\nclient_id=x\nclient_id=y",
            "haze-gdrive-credential-v1\nunknown=x",
        ] {
            let error = GoogleCredentials::parse(fixture).expect_err("must fail closed");
            assert_eq!(error.category(), AuthErrorCategory::InvalidCredentials);
            assert!(!error.is_retryable());
        }
    }

    #[test]
    fn expired_cached_token_causes_refresh() {
        let now = fixed_now();
        let endpoint = FakeTokenEndpoint {
            refreshes: 0,
            next: Some(Ok(token(
                "refreshed-access",
                now + Duration::from_secs(300),
            ))),
        };
        let mut client = auth_client(endpoint);
        client.access_token = Some(token(
            "expired-access",
            now.checked_sub(Duration::from_secs(1))
                .expect("fixed time supports subtraction"),
        ));

        let refreshed = client.access_token(now).expect("refresh should succeed");

        assert_eq!(refreshed.expose_for_authorization(), "refreshed-access");
        assert_eq!(client.endpoint.refreshes, 1);
    }

    #[test]
    fn usable_refreshed_token_is_cached_and_reused() {
        let now = fixed_now();
        let endpoint = FakeTokenEndpoint {
            refreshes: 0,
            next: Some(Ok(token(
                "refreshed-access",
                now + Duration::from_secs(300),
            ))),
        };
        let mut client = auth_client(endpoint);

        let first = client.access_token(now).expect("refresh should succeed");
        assert_eq!(first.expose_for_authorization(), "refreshed-access");
        let second = client
            .access_token(now + Duration::from_secs(10))
            .expect("cached token should remain usable");

        assert_eq!(second.expose_for_authorization(), "refreshed-access");
        assert_eq!(client.endpoint.refreshes, 1);
    }

    #[test]
    fn too_short_refreshed_token_is_rejected_and_not_cached() {
        let now = fixed_now();
        let endpoint = FakeTokenEndpoint {
            refreshes: 0,
            next: Some(Ok(token("too-short-access", now + Duration::from_secs(29)))),
        };
        let mut client = auth_client(endpoint);

        let error = client
            .access_token(now)
            .expect_err("short refreshed token must fail closed");

        assert_eq!(error.category(), AuthErrorCategory::RefreshUnavailable);
        assert!(error.is_retryable());
        assert!(client.access_token.is_none());
        assert_eq!(client.endpoint.refreshes, 1);
        assert!(!error.to_string().contains("too-short-access"));
    }

    #[test]
    fn valid_cached_token_is_returned_without_refresh() {
        let now = fixed_now();
        let mut client = auth_client(FakeTokenEndpoint::default());
        client.access_token = Some(token("cached-access", now + Duration::from_secs(300)));

        let cached = client
            .access_token(now)
            .expect("cached token should remain usable");

        assert_eq!(cached.expose_for_authorization(), "cached-access");
        assert!(!format!("{cached:?}").contains("cached-access"));
        assert_eq!(client.endpoint.refreshes, 0);
    }

    #[test]
    fn revoked_and_insufficient_scope_are_safe_categories() {
        let required = BTreeSet::from([DRIVE_SCOPE.to_owned()]);
        let insufficient = FakeGoogleProviderClient {
            scopes: BTreeSet::new(),
            root_accessible: true,
            error: None,
        };
        assert_eq!(
            preflight(&insufficient, &required, "root")
                .expect_err("scope must fail")
                .category(),
            AuthErrorCategory::InsufficientScope
        );

        let revoked = FakeGoogleProviderClient {
            scopes: required.clone(),
            root_accessible: false,
            error: None,
        };
        assert_eq!(
            preflight(&revoked, &required, "root")
                .expect_err("revoked must fail")
                .category(),
            AuthErrorCategory::Revoked
        );
    }

    #[test]
    fn retry_classification_is_explicit_and_provider_bodies_are_absent() {
        let error = AuthError::new(
            AuthErrorCategory::ProviderUnavailable,
            true,
            "Google provider is unavailable",
        );
        assert!(error.is_retryable());
        assert_eq!(error.to_string(), "Google provider is unavailable");
        assert!(!error.to_string().contains('{'));
    }

    #[test]
    fn successful_preflight_is_observation_only() {
        let required = BTreeSet::from([DRIVE_SCOPE.to_owned()]);
        let provider = FakeGoogleProviderClient {
            scopes: required.clone(),
            root_accessible: true,
            error: None,
        };
        let report = preflight(&provider, &required, "root").expect("preflight");
        assert!(report.root_accessible);
        assert!(report.scopes_satisfied);
    }
}
