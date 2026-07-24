//! Haze Sync server binary.

use std::{error::Error, fmt, fs, process::ExitCode, sync::Arc};

use tokio::net::TcpListener;

use crate::{
    config::{ConfigError, ObjectStoreConfig, ServerConfig},
    control_plane::{ControlPlaneError, ControlPlaneServices, DurableMaintenanceState},
    db::DbRuntimeError,
    state::ServerAppState,
    worktree_host::{ServerWorktreeHostConfig, ServerWorktreeHostError, ServerWorktreeRuntimeHost},
    worktree_http::ServerWorktreeHttpControl,
};

mod application;
pub mod config;
mod control_plane;
pub mod db;
pub mod http;
pub mod readiness;
pub mod routes;
pub mod state;
mod worktree_executor;
mod worktree_host;
mod worktree_http;
#[allow(dead_code)]
mod worktree_runtime;
mod worktree_status;

pub const CRATE_ROLE: &str =
    "Haze Sync HTTP server, operational control plane, and W2/W3 route wiring.";

#[must_use]
pub const fn package_name() -> &'static str {
    "haze-sync-server"
}

#[tokio::main]
async fn main() -> ExitCode {
    match run_from_env().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("haze-sync-server startup failed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run_from_env() -> Result<(), StartupError> {
    run_with_config(ServerConfig::from_env()?).await
}

async fn run_with_config(config: ServerConfig) -> Result<(), StartupError> {
    ensure_object_store_root(&config.object_store)?;
    let pool = db::connect_pg_pool(&config.database).await?;
    let listen_addr = config.listen_addr;
    let control_plane = ControlPlaneServices::load(
        pool.clone(),
        config.database.database_url.as_sensitive_str().as_bytes(),
    )
    .await?;
    control_plane.reconcile_startup().await?;
    let state = ServerAppState::from_config(config.clone(), pool.clone())
        .with_control_plane(control_plane.clone());

    let mut worktree_host = None;
    let state = if control_plane.maintenance_state() == DurableMaintenanceState::Normal {
        let services = state
            .application_services()
            .ok_or(StartupError::WorktreeHost(
                ServerWorktreeHostError::RuntimeFailed,
            ))?;
        let host_config = ServerWorktreeHostConfig::from_adapter_mode(config.worktree.mode)?;
        let manual_budget = host_config.action_budget;
        let host = Arc::new(
            ServerWorktreeRuntimeHost::start(
                host_config,
                config.worktree.root.clone(),
                pool,
                services,
            )
            .await?,
        );
        let control = ServerWorktreeHttpControl::new(Arc::downgrade(&host), manual_budget);
        worktree_host = Some(host);
        state.with_worktree_control(control)
    } else {
        state
    };
    let _legacy_status_boundary = ServerWorktreeRuntimeHost::status;
    let _readiness_boundary = crate::worktree_status::ServerWorktreeStatusSnapshot::is_ready;

    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|_error| StartupError::BindFailed)?;
    let serve_result = axum::serve(listener, routes::build_router_with_state(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|_error| StartupError::ServeFailed);
    let host_result = match worktree_host {
        Some(host) => match Arc::try_unwrap(host) {
            Ok(host) => host.shutdown().await.map(|_| ()),
            Err(_host) => Err(ServerWorktreeHostError::TaskFailed),
        },
        None => Ok(()),
    };

    match (serve_result, host_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn ensure_object_store_root(config: &ObjectStoreConfig) -> Result<(), StartupError> {
    fs::create_dir_all(&config.root).map_err(|_error| StartupError::ObjectStoreRootUnavailable)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StartupError {
    Config(ConfigError),
    Database(DbRuntimeError),
    ControlPlane(ControlPlaneError),
    WorktreeHost(ServerWorktreeHostError),
    ObjectStoreRootUnavailable,
    BindFailed,
    ServeFailed,
}

impl fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "configuration error: {error}"),
            Self::Database(error) => write!(formatter, "database startup failed: {}", error.code()),
            Self::ControlPlane(error) => {
                write!(
                    formatter,
                    "operational control startup failed: {}",
                    error.safe_code()
                )
            }
            Self::WorktreeHost(error) => write!(formatter, "worktree host failed: {error}"),
            Self::ObjectStoreRootUnavailable => {
                formatter.write_str("object store root could not be prepared")
            }
            Self::BindFailed => formatter.write_str("listener bind failed"),
            Self::ServeFailed => formatter.write_str("http server failed"),
        }
    }
}

impl Error for StartupError {}

impl From<ConfigError> for StartupError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<DbRuntimeError> for StartupError {
    fn from(error: DbRuntimeError) -> Self {
        Self::Database(error)
    }
}

impl From<ControlPlaneError> for StartupError {
    fn from(error: ControlPlaneError) -> Self {
        Self::ControlPlane(error)
    }
}

impl From<ServerWorktreeHostError> for StartupError {
    fn from(error: ServerWorktreeHostError) -> Self {
        Self::WorktreeHost(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-server");
    }

    #[test]
    fn startup_errors_are_secret_safe() {
        for rendered in [
            StartupError::WorktreeHost(ServerWorktreeHostError::BindingFailed).to_string(),
            StartupError::ControlPlane(ControlPlaneError::Internal).to_string(),
        ] {
            assert!(!rendered.contains("postgres://"));
            assert!(!rendered.contains("/srv/"));
            assert!(!rendered.contains("fingerprint"));
            assert!(!rendered.contains("secret"));
        }
    }
}
