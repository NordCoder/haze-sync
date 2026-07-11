//! Haze Sync server binary.
//!
//! Startup loads explicit runtime configuration, connects to PostgreSQL, creates
//! the local object-store root if needed, builds `ServerAppState`, and serves the
//! existing Axum router until graceful shutdown. It also owns the explicit
//! Worktree composition lifecycle, while real Worktree cycle execution remains
//! deferred to SRV-P7B.

use std::{error::Error, fmt, fs, future::Future, process::ExitCode};

use tokio::net::TcpListener;

use crate::{
    config::{ConfigError, ObjectStoreConfig, ServerConfig},
    db::DbRuntimeError,
    state::ServerAppState,
    worktree_runtime::{ServerWorktreeLifecycleError, ServerWorktreeRuntime},
};

mod application;
pub mod config;
pub mod db;
pub mod http;
pub mod readiness;
pub mod routes;
pub mod state;
mod worktree_runtime;

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
    let mut worktree_runtime =
        ServerWorktreeRuntime::new(config.worktree.mode, config.worktree.root.clone());
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

    let serve = async {
        axum::serve(listener, routes::build_router_with_state(state))
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|_error| StartupError::ServeFailed)
    };

    run_http_with_worktree_lifecycle(&mut worktree_runtime, serve).await
}

async fn run_http_with_worktree_lifecycle<F>(
    worktree_runtime: &mut ServerWorktreeRuntime,
    serve: F,
) -> Result<(), StartupError>
where
    F: Future<Output = Result<(), StartupError>>,
{
    let startup_status = worktree_runtime.start().map_err(StartupError::from)?;
    eprintln!("haze-sync-server worktree startup: {startup_status}");

    let serve_result = serve.await;
    let shutdown_result = worktree_runtime.shutdown().map_err(StartupError::from);
    if let Ok(shutdown_status) = &shutdown_result {
        eprintln!("haze-sync-server worktree shutdown: {shutdown_status}");
    }

    match (serve_result, shutdown_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(_)) => Ok(()),
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
    WorktreeLifecycle(ServerWorktreeLifecycleError),
    ObjectStoreRootUnavailable,
    BindFailed,
    ServeFailed,
}

impl fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "configuration error: {error}"),
            Self::Database(error) => write!(formatter, "database startup failed: {}", error.code()),
            Self::WorktreeLifecycle(error) => {
                write!(formatter, "worktree lifecycle failed: {error}")
            }
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

impl From<ServerWorktreeLifecycleError> for StartupError {
    fn from(error: ServerWorktreeLifecycleError) -> Self {
        Self::WorktreeLifecycle(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haze_sync_common::AdapterMode;
    use std::path::PathBuf;

    #[test]
    fn package_name_matches_crate() {
        assert_eq!(package_name(), "haze-sync-server");
    }

    #[tokio::test]
    async fn worktree_lifecycle_closes_after_successful_serve() {
        let mut runtime =
            ServerWorktreeRuntime::new(AdapterMode::Disabled, PathBuf::from("./unused"));

        run_http_with_worktree_lifecycle(&mut runtime, async { Ok(()) })
            .await
            .expect("successful serve should close cleanly");

        assert_eq!(
            runtime.status().lifecycle(),
            crate::worktree_runtime::ServerWorktreeLifecycle::Shutdown
        );
    }

    #[tokio::test]
    async fn worktree_lifecycle_closes_after_failed_serve() {
        let mut runtime =
            ServerWorktreeRuntime::new(AdapterMode::Disabled, PathBuf::from("./unused"));

        let result = run_http_with_worktree_lifecycle(&mut runtime, async {
            Err(StartupError::ServeFailed)
        })
        .await;

        assert_eq!(result, Err(StartupError::ServeFailed));
        assert_eq!(
            runtime.status().lifecycle(),
            crate::worktree_runtime::ServerWorktreeLifecycle::Shutdown
        );
    }

    #[tokio::test]
    async fn worktree_lifecycle_errors_are_secret_safe() {
        let secret_root = PathBuf::from("/srv/private/token-like-worktree-root");
        let mut runtime = ServerWorktreeRuntime::new(AdapterMode::Disabled, secret_root.clone());
        runtime.start().expect("test setup should start once");

        let error = run_http_with_worktree_lifecycle(&mut runtime, async { Ok(()) })
            .await
            .expect_err("second start should fail safely");
        let message = error.to_string();

        assert_eq!(
            error,
            StartupError::WorktreeLifecycle(ServerWorktreeLifecycleError::AlreadyStarted)
        );
        assert!(!message.contains(secret_root.to_string_lossy().as_ref()));
        assert!(!message.contains("/srv/private"));
        assert!(!message.contains("token-like"));
    }
}
