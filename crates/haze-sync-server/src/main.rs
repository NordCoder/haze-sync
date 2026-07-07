//! Haze Sync server binary.
//!
//! Startup loads explicit runtime configuration, connects to PostgreSQL, creates
//! the local object-store root if needed, builds `ServerAppState`, and serves the
//! existing Axum router until graceful shutdown. It does not auto-run migrations,
//! start adapter loops, call providers, or perform worktree runtime behavior.

use std::{error::Error, fmt, fs, process::ExitCode};

use tokio::net::TcpListener;

use crate::{
    config::{ConfigError, ObjectStoreConfig, ServerConfig},
    db::DbRuntimeError,
    state::ServerAppState,
};

pub mod config;
pub mod db;
pub mod http;
pub mod readiness;
pub mod routes;
pub mod state;

/// Human-readable crate role used by smoke checks and documentation.
pub const CRATE_ROLE: &str =
    "Haze Sync HTTP server scaffolding, configuration, DB readiness, migrations, and W2/W3 route wiring.";

/// Returns the package name for this crate.
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

    // Migrations are deliberately not run here. They remain an explicit operator
    // action through db::migrations::run_repository_migrations until a later
    // accepted startup policy scopes automatic migration behavior.
    let pool = db::connect_pg_pool(&config.database).await?;
    let listen_addr = config.listen_addr;
    let state = ServerAppState::from_config(config, pool);
    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|_error| StartupError::BindFailed)?;

    axum::serve(listener, routes::build_router_with_state(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|_error| StartupError::ServeFailed)
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
    ObjectStoreRootUnavailable,
    BindFailed,
    ServeFailed,
}

impl fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "configuration error: {error}"),
            Self::Database(error) => write!(formatter, "database startup failed: {}", error.code()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-server");
    }
}
