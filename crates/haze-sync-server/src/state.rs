//! Server application state for W2 Core file-operation wiring.
//!
//! The state is explicit and cloneable for Axum handlers. It carries caller-owned
//! runtime dependencies only; route construction still supports dependency-free
//! health/readiness tests without hidden globals.

use haze_sync_api::auth::AdapterPrincipal;
use haze_sync_storage::LocalObjectStore;
use sqlx::PgPool;

use crate::{
    application::ServerApplicationServices,
    config::{ObjectStoreConfig, ServerConfig},
    readiness::ReadinessState,
};

/// Explicit server dependencies used by W2 Core file-operation routes.
#[derive(Clone)]
pub struct ServerAppState {
    db_pool: Option<PgPool>,
    object_store: Option<LocalObjectStore>,
    config: Option<ServerConfig>,
    auth: AuthState,
}

impl ServerAppState {
    /// Safe dependency-free state used by route-shell tests and `/health`.
    #[must_use]
    pub const fn dependency_free() -> Self {
        Self {
            db_pool: None,
            object_store: None,
            config: None,
            auth: AuthState::Disabled,
        }
    }

    /// Build runtime state from already-created dependencies.
    #[must_use]
    pub fn new(
        db_pool: Option<PgPool>,
        object_store: Option<LocalObjectStore>,
        config: Option<ServerConfig>,
        auth: AuthState,
    ) -> Self {
        Self {
            db_pool,
            object_store,
            config,
            auth,
        }
    }

    /// Build state from server config and a caller-owned PostgreSQL pool.
    #[must_use]
    pub fn from_config(config: ServerConfig, db_pool: PgPool) -> Self {
        let object_store = LocalObjectStore::new(config.object_store.root.clone());
        Self {
            db_pool: Some(db_pool.clone()),
            object_store: Some(object_store),
            config: Some(config),
            auth: AuthState::Database { pool: db_pool },
        }
    }

    /// Build state for deterministic handler tests with a static principal.
    #[must_use]
    pub fn with_static_principal(principal: AdapterPrincipal) -> Self {
        Self {
            db_pool: None,
            object_store: None,
            config: None,
            auth: AuthState::StaticPrincipal { principal },
        }
    }

    /// Return the configured database pool, when runtime-backed routes are enabled.
    #[must_use]
    pub const fn db_pool(&self) -> Option<&PgPool> {
        self.db_pool.as_ref()
    }

    /// Return the configured local object store, when runtime-backed routes are enabled.
    #[must_use]
    pub const fn object_store(&self) -> Option<&LocalObjectStore> {
        self.object_store.as_ref()
    }

    /// Build reusable application services when the database authority exists.
    ///
    /// File PUT/GET services validate object-store availability per operation;
    /// DELETE and changes preserve their existing database-only dependency shape.
    #[must_use]
    pub(crate) fn application_services(&self) -> Option<ServerApplicationServices> {
        Some(ServerApplicationServices::new(
            self.db_pool.as_ref()?.clone(),
            self.object_store.clone(),
        ))
    }

    /// Return the loaded server configuration, when startup supplied one.
    #[must_use]
    pub const fn config(&self) -> Option<&ServerConfig> {
        self.config.as_ref()
    }

    /// Return explicit authentication state.
    #[must_use]
    pub const fn auth(&self) -> &AuthState {
        &self.auth
    }

    /// Build a readiness state from the same explicit dependencies.
    #[must_use]
    pub fn readiness_state(&self) -> ReadinessState {
        ReadinessState::from_optional(
            self.db_pool.clone(),
            self.object_store.as_ref().map(|store| ObjectStoreConfig {
                root: store.root().to_path_buf(),
            }),
        )
    }
}

impl Default for ServerAppState {
    fn default() -> Self {
        Self::dependency_free()
    }
}

impl std::fmt::Debug for ServerAppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServerAppState")
            .field("db_pool", &self.db_pool.as_ref().map(|_pool| "[REDACTED]"))
            .field(
                "object_store",
                &self.object_store.as_ref().map(|_store| "[REDACTED]"),
            )
            .field("config", &self.config.as_ref().map(|_config| "[REDACTED]"))
            .field("auth", &self.auth)
            .finish()
    }
}

/// Authentication source used by server routes.
#[derive(Clone)]
pub enum AuthState {
    /// No auth source was configured. Protected routes reject requests safely.
    Disabled,
    /// Deterministic test principal; a syntactically valid bearer token is still required.
    StaticPrincipal { principal: AdapterPrincipal },
    /// Runtime token lookup through `sync_adapters`.
    Database { pool: PgPool },
}

impl std::fmt::Debug for AuthState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("AuthState::Disabled"),
            Self::StaticPrincipal { principal } => formatter
                .debug_struct("AuthState::StaticPrincipal")
                .field("adapter_id", &principal.adapter_id())
                .field("role", &principal.role())
                .finish(),
            Self::Database { .. } => formatter.write_str("AuthState::Database([REDACTED])"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_api::auth::AdapterRole;

    #[tokio::test]
    async fn dependency_free_state_is_not_ready_without_leaks() {
        let state = ServerAppState::dependency_free();
        let report = state.readiness_state().check().await;
        let rendered = format!("{state:?} {report:?}");

        assert!(!report.is_ready());
        assert!(state.application_services().is_none());
        assert!(!rendered.contains("postgres://"));
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("/srv/"));
    }

    #[test]
    fn static_principal_state_redacts_runtime_handles() {
        let principal =
            AdapterPrincipal::new("obsidian-plugin", AdapterRole::ObsidianPlugin).unwrap();
        let rendered = format!("{:?}", ServerAppState::with_static_principal(principal));

        assert!(rendered.contains("obsidian-plugin"));
        assert!(!rendered.contains("Bearer"));
        assert!(!rendered.contains("token"));
    }
}
