//! Haze Sync server binary.
//!
//! Startup loads explicit configuration, connects to PostgreSQL, prepares the
//! object store, constructs one joined Worktree runtime host, and serves Axum
//! until graceful shutdown. The Worktree host remains internal to Server.

use std::{error::Error, fmt, fs, process::ExitCode};

use tokio::net::TcpListener;

use crate::{
    config::{ConfigError, ObjectStoreConfig, ServerConfig},
    db::DbRuntimeError,
    state::ServerAppState,
    worktree_host::{ServerWorktreeHostConfig, ServerWorktreeHostError, ServerWorktreeRuntimeHost},
};

mod application;
pub mod config;
pub mod db;
pub mod http;
pub mod readiness;
pub mod routes;
pub mod state;
mod worktree_executor;
mod worktree_host;
#[allow(dead_code)]
mod worktree_runtime;
mod worktree_status;

pub const CRATE_ROLE: &str =
    "Haze Sync HTTP server scaffolding, configuration, DB readiness, migrations, and W2/W3 route wiring.";

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
    let state = ServerAppState::from_config(config.clone(), pool.clone());
    let services = state
        .application_services()
        .ok_or(StartupError::WorktreeHost(
            ServerWorktreeHostError::RuntimeFailed,
        ))?;
    let host_config = ServerWorktreeHostConfig::from_adapter_mode(config.worktree.mode)?;
    let worktree_host =
        ServerWorktreeRuntimeHost::start(host_config, config.worktree.root.clone(), pool, services)
            .await?;
    let _manual_submission_boundary = ServerWorktreeRuntimeHost::submit_manual;
    let _status_snapshot_boundary = ServerWorktreeRuntimeHost::snapshot;

    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|_error| StartupError::BindFailed)?;
    let serve_result = axum::serve(listener, routes::build_router_with_state(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|_error| StartupError::ServeFailed);
    let host_result = worktree_host.shutdown().await.map(|_| ());

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
    fn worktree_host_errors_are_secret_safe() {
        let rendered =
            StartupError::WorktreeHost(ServerWorktreeHostError::BindingFailed).to_string();
        assert!(!rendered.contains("postgres://"));
        assert!(!rendered.contains("/srv/"));
        assert!(!rendered.contains("fingerprint"));
    }
}
