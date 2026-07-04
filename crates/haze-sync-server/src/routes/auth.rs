use axum::http::HeaderMap;
use haze_sync_api::auth::{AdapterPrincipal, AdapterRole, BearerToken};
use haze_sync_api::contracts::headers::AUTHORIZATION_HEADER;
use sqlx::{PgPool, Row};
use std::str::FromStr;

use crate::state::{AuthState, ServerAppState};

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
    let header = required_auth_header(headers)?;
    let token = BearerToken::parse_authorization_header(header)
        .map_err(|_error| AuthFailure::InvalidToken)?;

    match state.auth() {
        AuthState::Disabled => Err(AuthFailure::InvalidToken),
        AuthState::StaticPrincipal { principal } => Ok(principal.clone()),
        AuthState::Database { pool } => lookup_principal_by_token(pool, &token).await,
    }
}

fn required_auth_header(headers: &HeaderMap) -> Result<&str, AuthFailure> {
    let value = headers
        .get(AUTHORIZATION_HEADER)
        .ok_or(AuthFailure::MissingToken)?;
    value.to_str().map_err(|_error| AuthFailure::InvalidToken)
}

async fn lookup_principal_by_token(
    pool: &PgPool,
    token: &BearerToken,
) -> Result<AdapterPrincipal, AuthFailure> {
    let hash = token.sha256_hash();
    let prefixed_hash = format!("sha256:{}", hash.digest_hex());
    let row = sqlx::query(
        "select adapter_id, role from sync_adapters \
         where enabled = true and (token_hash = $1 or token_hash = $2) \
         limit 1",
    )
    .bind(hash.digest_hex())
    .bind(prefixed_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_error| AuthFailure::Internal)?
    .ok_or(AuthFailure::InvalidToken)?;

    let adapter_id: String = row
        .try_get("adapter_id")
        .map_err(|_error| AuthFailure::Internal)?;
    let role: String = row
        .try_get("role")
        .map_err(|_error| AuthFailure::Internal)?;
    let role = AdapterRole::from_str(&role).map_err(|_error| AuthFailure::InvalidToken)?;

    AdapterPrincipal::new(adapter_id, role).map_err(|_error| AuthFailure::InvalidToken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::auth::AdapterRole;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

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

    #[tokio::test]
    async fn database_auth_attempts_runtime_lookup_instead_of_short_circuiting() {
        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://haze_sync:placeholder@127.0.0.1:1/haze_sync_test")
            .expect("lazy pool should build");
        let state = ServerAppState::new(None, None, None, AuthState::Database { pool });

        let error = authenticate_principal(&state, &bearer_headers())
            .await
            .expect_err("unreachable db lookup should fail safely");

        assert_eq!(error, AuthFailure::Internal);
    }
}
