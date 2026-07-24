use axum::http::HeaderMap;
use haze_sync_api::auth::{AdapterPrincipal, BearerToken};
use haze_sync_api::contracts::headers::AUTHORIZATION_HEADER;

use crate::{
    control_plane::{AuthenticatedIdentity, ControlPlaneError},
    state::{AuthState, ServerAppState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AuthFailure {
    MissingToken,
    InvalidToken,
    Internal,
}

pub(super) async fn authenticate_principal(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AdapterPrincipal, AuthFailure> {
    authenticate_identity(state, headers)
        .await
        .map(|identity| identity.principal)
}

pub(super) async fn authenticate_identity(
    state: &ServerAppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedIdentity, AuthFailure> {
    let header = required_auth_header(headers)?;
    let token = BearerToken::parse_authorization_header(header)
        .map_err(|_error| AuthFailure::InvalidToken)?;
    match state.auth() {
        AuthState::Disabled => Err(AuthFailure::InvalidToken),
        AuthState::StaticPrincipal { principal } => {
            Ok(AuthenticatedIdentity::static_principal(principal.clone()))
        }
        AuthState::Database { .. } => state
            .control_plane()
            .ok_or(AuthFailure::Internal)?
            .authenticate_bearer(&token)
            .await
            .map_err(map_control_auth_error),
    }
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, AuthFailure> {
    let value = headers
        .get(AUTHORIZATION_HEADER)
        .ok_or(AuthFailure::MissingToken)?;
    value.to_str().map_err(|_error| AuthFailure::InvalidToken)
}

fn map_control_auth_error(error: ControlPlaneError) -> AuthFailure {
    match error {
        ControlPlaneError::Internal | ControlPlaneError::InvalidStoredState => AuthFailure::Internal,
        _ => AuthFailure::InvalidToken,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::auth::AdapterRole;

    fn bearer_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION_HEADER,
            "Bearer route_test_token"
                .parse()
                .expect("header should parse"),
        );
        headers
    }

    #[tokio::test]
    async fn disabled_auth_rejects_even_valid_bearer_headers() {
        let error = authenticate_principal(&ServerAppState::dependency_free(), &bearer_headers())
            .await
            .expect_err("disabled auth should reject");
        assert_eq!(error, AuthFailure::InvalidToken);
    }

    #[tokio::test]
    async fn static_principal_auth_returns_the_configured_principal() {
        let principal = AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin)
            .expect("fixture principal should parse");
        let state = ServerAppState::with_static_principal(principal.clone());
        let authenticated = authenticate_principal(&state, &bearer_headers())
            .await
            .expect("static principal should authenticate");
        assert_eq!(authenticated, principal);
    }

    #[tokio::test]
    async fn malformed_bearer_headers_are_rejected_before_state_lookup() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION_HEADER,
            "Token route_test_token"
                .parse()
                .expect("header should parse"),
        );
        let error = authenticate_principal(&ServerAppState::dependency_free(), &headers)
            .await
            .expect_err("malformed token should reject");
        assert_eq!(error, AuthFailure::InvalidToken);
    }
}
