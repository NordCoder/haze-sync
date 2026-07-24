//! Explicit cloneable Server runtime state.

use haze_sync_api::auth::AdapterPrincipal;
use haze_sync_storage::LocalObjectStore;
use sqlx::PgPool;

use crate::{
    application::ServerApplicationServices,
    config::{ObjectStoreConfig, ServerConfig},
    control_plane::ControlPlaneServices,
    readiness::ReadinessState,
    worktree_http::ServerWorktreeHttpControl,
};

#[derive(Clone)]
pub struct ServerAppState {
    db_pool: Option<PgPool>,
    object_store: Option<LocalObjectStore>,
    config: Option<ServerConfig>,
    auth: AuthState,
    worktree_control: Option<ServerWorktreeHttpControl>,
    control_plane: Option<ControlPlaneServices>,
}

impl ServerAppState {
    #[must_use]
    pub const fn dependency_free() -> Self {
        Self {
            db_pool: None,
            object_store: None,
            config: None,
            auth: AuthState::Disabled,
            worktree_control: None,
            control_plane: None,
        }
    }

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
            worktree_control: None,
            control_plane: None,
        }
    }

    #[must_use]
    pub fn from_config(config: ServerConfig, db_pool: PgPool) -> Self {
        let object_store = LocalObjectStore::new(config.object_store.root.clone());
        Self {
            db_pool: Some(db_pool.clone()),
            object_store: Some(object_store),
            config: Some(config),
            auth: AuthState::Database { pool: db_pool.clone() },
            worktree_control: None,
            control_plane: None,
        }
    }

    #[must_use]
    pub fn with_static_principal(principal: AdapterPrincipal) -> Self {
        Self {
            db_pool: None,
            object_store: None,
            config: None,
            auth: AuthState::StaticPrincipal { principal },
            worktree_control: None,
            control_plane: None,
        }
    }

    #[must_use]
    pub(crate) fn with_worktree_control(mut self, control: ServerWorktreeHttpControl) -> Self {
        self.worktree_control = Some(control);
        self
    }

    #[must_use]
    pub(crate) fn with_control_plane(mut self, control_plane: ControlPlaneServices) -> Self {
        self.control_plane = Some(control_plane);
        self
    }

    #[must_use]
    pub const fn db_pool(&self) -> Option<&PgPool> {
        self.db_pool.as_ref()
    }

    #[must_use]
    pub const fn object_store(&self) -> Option<&LocalObjectStore> {
        self.object_store.as_ref()
    }

    #[must_use]
    pub(crate) fn application_services(&self) -> Option<ServerApplicationServices> {
        let pool = self.db_pool.as_ref()?.clone();
        Some(match &self.control_plane {
            Some(control_plane) => ServerApplicationServices::new_with_admission(
                pool,
                self.object_store.clone(),
                control_plane.admission().clone(),
            ),
            None => ServerApplicationServices::new(pool, self.object_store.clone()),
        })
    }

    #[must_use]
    pub const fn config(&self) -> Option<&ServerConfig> {
        self.config.as_ref()
    }

    #[must_use]
    pub const fn auth(&self) -> &AuthState {
        &self.auth
    }

    #[must_use]
    pub(crate) const fn worktree_control(&self) -> Option<&ServerWorktreeHttpControl> {
        self.worktree_control.as_ref()
    }

    #[must_use]
    pub(crate) const fn control_plane(&self) -> Option<&ControlPlaneServices> {
        self.control_plane.as_ref()
    }

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
            .field("worktree_control", &self.worktree_control.is_some())
            .field("control_plane", &self.control_plane.is_some())
            .finish()
    }
}

#[derive(Clone)]
pub enum AuthState {
    Disabled,
    StaticPrincipal { principal: AdapterPrincipal },
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
        assert!(state.worktree_control().is_none());
        assert!(state.control_plane().is_none());
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
