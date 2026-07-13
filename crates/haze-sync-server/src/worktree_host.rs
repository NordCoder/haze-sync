//! Server-owned hosting for the accepted Worktree runtime contract.

use crate::{
    application::ServerApplicationServices,
    worktree_executor::{
        ServerWorktreeCycleExecutor, ServerWorktreeExecutorConfigError,
        ServerWorktreeExecutorPolicy,
    },
    worktree_runtime::{map_adapter_mode, ServerWorktreeModeMapping},
};
use haze_sync_common::{AdapterId, AdapterMode, Sha256};
use haze_sync_storage::repositories::worktree_state::{
    bind_or_verify_worktree_instance, WorktreeInstanceBinding,
};
use haze_sync_worktree::{
    ProductionWorktreeWatcher, WorktreeConfig, WorktreeHostedRuntime, WorktreeHostedRuntimeError,
    WorktreeHostedRuntimePoll, WorktreeMode, WorktreeRuntimeClock, WorktreeRuntimeCycle,
    WorktreeRuntimeCycleBudget, WorktreeRuntimeLifecycle, WorktreeRuntimeManualHandle,
    WorktreeRuntimeManualRequest, WorktreeRuntimeManualSubmission, WorktreeRuntimePolicy,
    WorktreeRuntimePolicyError, WorktreeRuntimeService, WorktreeRuntimeStatus,
    WorktreeRuntimeWatcherState, WorktreeWatcher, WorktreeWatcherFailure,
};
use sha2::{Digest, Sha256 as Sha256Hasher};
use sqlx::PgPool;
use std::{fmt, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    sync::{oneshot, RwLock},
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};

const MAX_ACTION_BUDGET: usize = 1_000;
const MAX_MANUAL_CAPACITY: usize = 64;
const MAX_WATCHER_HINT_BUDGET: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ServerWorktreeHostConfig {
    pub(crate) mode: WorktreeMode,
    pub(crate) debounce: Duration,
    pub(crate) periodic_interval: Duration,
    pub(crate) watcher_hint_budget: usize,
    pub(crate) action_budget: WorktreeRuntimeCycleBudget,
    pub(crate) shutdown_timeout: Duration,
    pub(crate) manual_capacity: usize,
}

impl ServerWorktreeHostConfig {
    pub(crate) fn from_adapter_mode(mode: AdapterMode) -> Result<Self, ServerWorktreeHostError> {
        let mode = match map_adapter_mode(mode) {
            ServerWorktreeModeMapping::Supported(mode) => mode,
            ServerWorktreeModeMapping::UnsupportedDryRun => WorktreeMode::DryRun,
        };
        Self::new(
            mode,
            Duration::from_millis(250),
            Duration::from_secs(30),
            64,
            WorktreeRuntimeCycleBudget {
                max_import_actions: 250,
                max_delete_candidates: 250,
                max_export_actions: 250,
            },
            Duration::from_secs(10),
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        mode: WorktreeMode,
        debounce: Duration,
        periodic_interval: Duration,
        watcher_hint_budget: usize,
        action_budget: WorktreeRuntimeCycleBudget,
        shutdown_timeout: Duration,
        manual_capacity: usize,
    ) -> Result<Self, ServerWorktreeHostError> {
        if periodic_interval.is_zero()
            || watcher_hint_budget == 0
            || watcher_hint_budget > MAX_WATCHER_HINT_BUDGET
            || action_budget.max_import_actions == 0
            || action_budget.max_import_actions > MAX_ACTION_BUDGET
            || action_budget.max_delete_candidates == 0
            || action_budget.max_delete_candidates > MAX_ACTION_BUDGET
            || action_budget.max_export_actions == 0
            || action_budget.max_export_actions > MAX_ACTION_BUDGET
            || shutdown_timeout.is_zero()
            || manual_capacity == 0
            || manual_capacity > MAX_MANUAL_CAPACITY
        {
            return Err(ServerWorktreeHostError::InvalidConfig);
        }
        WorktreeRuntimePolicy::new(
            debounce,
            periodic_interval,
            watcher_hint_budget,
            action_budget,
        )
        .map_err(|_| ServerWorktreeHostError::InvalidConfig)?;
        Ok(Self {
            mode,
            debounce,
            periodic_interval,
            watcher_hint_budget,
            action_budget,
            shutdown_timeout,
            manual_capacity,
        })
    }

    fn policy(self) -> Result<WorktreeRuntimePolicy, WorktreeRuntimePolicyError> {
        WorktreeRuntimePolicy::new(
            self.debounce,
            self.periodic_interval,
            self.watcher_hint_budget,
            self.action_budget,
        )
    }

    fn poll_interval(self) -> Duration {
        let candidate = if self.debounce.is_zero() {
            Duration::from_millis(25)
        } else {
            self.debounce.min(Duration::from_millis(100))
        };
        candidate.min(self.periodic_interval)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeHostLifecycle {
    Disabled,
    Starting,
    Running,
    Cancelling,
    Shutdown,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ServerWorktreeHostStatus {
    pub(crate) lifecycle: ServerWorktreeHostLifecycle,
    pub(crate) cycles_completed: u64,
    pub(crate) cycles_failed: u64,
    pub(crate) pending_watcher_hints: usize,
    pub(crate) cycle_in_progress: bool,
}

impl ServerWorktreeHostStatus {
    const fn disabled() -> Self {
        Self {
            lifecycle: ServerWorktreeHostLifecycle::Disabled,
            cycles_completed: 0,
            cycles_failed: 0,
            pending_watcher_hints: 0,
            cycle_in_progress: false,
        }
    }

    const fn starting() -> Self {
        Self {
            lifecycle: ServerWorktreeHostLifecycle::Starting,
            cycles_completed: 0,
            cycles_failed: 0,
            pending_watcher_hints: 0,
            cycle_in_progress: false,
        }
    }

    fn failed_from(status: WorktreeRuntimeStatus) -> Self {
        Self {
            lifecycle: ServerWorktreeHostLifecycle::Failed,
            ..Self::from_runtime(status)
        }
    }

    fn from_runtime(status: WorktreeRuntimeStatus) -> Self {
        Self {
            lifecycle: match status.lifecycle {
                WorktreeRuntimeLifecycle::Created => ServerWorktreeHostLifecycle::Starting,
                WorktreeRuntimeLifecycle::Running => ServerWorktreeHostLifecycle::Running,
                WorktreeRuntimeLifecycle::Cancelling => ServerWorktreeHostLifecycle::Cancelling,
                WorktreeRuntimeLifecycle::Shutdown => ServerWorktreeHostLifecycle::Shutdown,
            },
            cycles_completed: status.cycles_completed,
            cycles_failed: status.cycles_failed,
            pending_watcher_hints: status.pending_watcher_hints,
            cycle_in_progress: status.cycle_in_progress,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ServerWorktreeHostError {
    InvalidConfig,
    InvalidRoot,
    BindingFailed,
    WatcherFailed,
    ExecutorFailed,
    RuntimeFailed,
    TaskFailed,
    StartupTimedOut,
    ShutdownTimedOut,
}

impl fmt::Display for ServerWorktreeHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfig => "worktree host configuration is invalid",
            Self::InvalidRoot => "worktree host root is invalid",
            Self::BindingFailed => "worktree host binding failed",
            Self::WatcherFailed => "worktree watcher construction failed",
            Self::ExecutorFailed => "worktree executor construction failed",
            Self::RuntimeFailed => "worktree hosted runtime failed",
            Self::TaskFailed => "worktree host task failed",
            Self::StartupTimedOut => "worktree host startup timed out",
            Self::ShutdownTimedOut => "worktree host shutdown timed out",
        })
    }
}

impl std::error::Error for ServerWorktreeHostError {}

impl From<ServerWorktreeExecutorConfigError> for ServerWorktreeHostError {
    fn from(_: ServerWorktreeExecutorConfigError) -> Self {
        Self::ExecutorFailed
    }
}

impl From<WorktreeWatcherFailure> for ServerWorktreeHostError {
    fn from(_: WorktreeWatcherFailure) -> Self {
        Self::WatcherFailed
    }
}

impl From<WorktreeHostedRuntimeError> for ServerWorktreeHostError {
    fn from(_: WorktreeHostedRuntimeError) -> Self {
        Self::RuntimeFailed
    }
}

#[must_use = "the host must be shut down and joined"]
pub(crate) struct ServerWorktreeRuntimeHost {
    manual: Option<WorktreeRuntimeManualHandle>,
    status: Arc<RwLock<ServerWorktreeHostStatus>>,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<Result<(), ServerWorktreeHostError>>>,
    shutdown_timeout: Duration,
}

impl fmt::Debug for ServerWorktreeRuntimeHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerWorktreeRuntimeHost")
            .field("hosted", &self.join.is_some())
            .field("manual_available", &self.manual.is_some())
            .finish()
    }
}

impl ServerWorktreeRuntimeHost {
    pub(crate) async fn start(
        config: ServerWorktreeHostConfig,
        root: PathBuf,
        pool: PgPool,
        services: ServerApplicationServices,
    ) -> Result<Self, ServerWorktreeHostError> {
        if config.mode == WorktreeMode::Disabled {
            return Ok(Self::disabled(config.shutdown_timeout));
        }

        let canonical_root = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&root).map_err(|_| ServerWorktreeHostError::InvalidRoot)?;
            std::fs::canonicalize(root).map_err(|_| ServerWorktreeHostError::InvalidRoot)
        })
        .await
        .map_err(|_| ServerWorktreeHostError::InvalidRoot)??;
        let adapter_id =
            AdapterId::parse("worktree").map_err(|_| ServerWorktreeHostError::InvalidConfig)?;
        let root_fingerprint = fingerprint_root(&canonical_root);
        bind_or_verify_worktree_instance(
            &pool,
            &WorktreeInstanceBinding::new(adapter_id.clone(), root_fingerprint),
        )
        .await
        .map_err(|_| ServerWorktreeHostError::BindingFailed)?;

        let worktree_config = WorktreeConfig::new(canonical_root.clone())
            .map_err(|_| ServerWorktreeHostError::InvalidRoot)?;
        let watcher = ProductionWorktreeWatcher::new(&worktree_config, config.watcher_hint_budget)?;
        let executor = ServerWorktreeCycleExecutor::new(
            services,
            pool,
            adapter_id,
            canonical_root,
            root_fingerprint,
            ServerWorktreeExecutorPolicy::default(),
        )?;
        let service = WorktreeRuntimeService::new(
            config.mode,
            config
                .policy()
                .map_err(|_| ServerWorktreeHostError::InvalidConfig)?,
            TokioWorktreeClock::new(),
            watcher,
            executor,
        );
        let (runtime, manual) = WorktreeHostedRuntime::new(service, config.manual_capacity)?;
        Self::start_runtime(runtime, manual, config).await
    }

    fn disabled(shutdown_timeout: Duration) -> Self {
        Self {
            manual: None,
            status: Arc::new(RwLock::new(ServerWorktreeHostStatus::disabled())),
            shutdown: None,
            join: None,
            shutdown_timeout,
        }
    }

    async fn start_runtime<C, W, X>(
        runtime: WorktreeHostedRuntime<C, W, X>,
        manual: WorktreeRuntimeManualHandle,
        config: ServerWorktreeHostConfig,
    ) -> Result<Self, ServerWorktreeHostError>
    where
        C: WorktreeRuntimeClock + Send + 'static,
        W: WorktreeWatcher + Send + 'static,
        X: WorktreeRuntimeCycle + Send + 'static,
    {
        let status = Arc::new(RwLock::new(ServerWorktreeHostStatus::starting()));
        let task_status = status.clone();
        let (shutdown, shutdown_receiver) = oneshot::channel();
        let (startup, startup_receiver) = oneshot::channel();
        let mut join = tokio::spawn(run_hosted(
            runtime,
            shutdown_receiver,
            startup,
            task_status,
            config.poll_interval(),
        ));

        match time::timeout(config.shutdown_timeout, startup_receiver).await {
            Ok(Ok(Ok(()))) => Ok(Self {
                manual: Some(manual),
                status,
                shutdown: Some(shutdown),
                join: Some(join),
                shutdown_timeout: config.shutdown_timeout,
            }),
            Ok(Ok(Err(error))) => {
                join_failed_start(&mut join, config.shutdown_timeout).await;
                Err(error)
            }
            Ok(Err(_)) => {
                join_failed_start(&mut join, config.shutdown_timeout).await;
                Err(ServerWorktreeHostError::TaskFailed)
            }
            Err(_) => {
                let _ = shutdown.send(());
                join_failed_start(&mut join, config.shutdown_timeout).await;
                Err(ServerWorktreeHostError::StartupTimedOut)
            }
        }
    }

    pub(crate) fn submit_manual(
        &self,
        request: WorktreeRuntimeManualRequest,
    ) -> WorktreeRuntimeManualSubmission {
        self.manual
            .as_ref()
            .map_or(WorktreeRuntimeManualSubmission::Shutdown, |manual| {
                manual.submit(request)
            })
    }

    pub(crate) async fn status(&self) -> ServerWorktreeHostStatus {
        *self.status.read().await
    }

    pub(crate) async fn shutdown(
        mut self,
    ) -> Result<ServerWorktreeHostStatus, ServerWorktreeHostError> {
        if self.join.is_none() {
            return Ok(self.status().await);
        }
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let mut join = self.join.take().expect("join existence checked above");
        match time::timeout(self.shutdown_timeout, &mut join).await {
            Ok(Ok(result)) => result?,
            Ok(Err(_)) => return Err(ServerWorktreeHostError::TaskFailed),
            Err(_) => {
                join.abort();
                let _ = join.await;
                let previous = self.status().await;
                *self.status.write().await = ServerWorktreeHostStatus {
                    lifecycle: ServerWorktreeHostLifecycle::Failed,
                    ..previous
                };
                return Err(ServerWorktreeHostError::ShutdownTimedOut);
            }
        }
        Ok(self.status().await)
    }
}

impl Drop for ServerWorktreeRuntimeHost {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(join) = self.join.take() {
            join.abort();
        }
    }
}

async fn join_failed_start(
    join: &mut JoinHandle<Result<(), ServerWorktreeHostError>>,
    timeout: Duration,
) {
    if time::timeout(timeout, &mut *join).await.is_err() {
        join.abort();
        let _ = join.await;
    }
}

async fn run_hosted<C, W, X>(
    mut runtime: WorktreeHostedRuntime<C, W, X>,
    mut shutdown: oneshot::Receiver<()>,
    startup: oneshot::Sender<Result<(), ServerWorktreeHostError>>,
    status: Arc<RwLock<ServerWorktreeHostStatus>>,
    poll_interval: Duration,
) -> Result<(), ServerWorktreeHostError>
where
    C: WorktreeRuntimeClock + Send + 'static,
    W: WorktreeWatcher + Send + 'static,
    X: WorktreeRuntimeCycle + Send + 'static,
{
    let start_failed = runtime.start().is_err()
        || matches!(
            runtime.status().watcher,
            WorktreeRuntimeWatcherState::Failed(_)
        );
    if start_failed {
        *status.write().await = ServerWorktreeHostStatus::failed_from(runtime.status());
        let _ = startup.send(Err(ServerWorktreeHostError::RuntimeFailed));
        return Err(ServerWorktreeHostError::RuntimeFailed);
    }
    publish_status(&status, runtime.status()).await;
    if startup.send(Ok(())).is_err() {
        cancel_and_shutdown(&mut runtime, &status).await?;
        return Err(ServerWorktreeHostError::TaskFailed);
    }

    let mut ticker = time::interval(poll_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => {
                cancel_and_shutdown(&mut runtime, &status).await?;
                return Ok(());
            }
            _ = ticker.tick() => {
                let poll_result = tokio::select! {
                    biased;
                    _ = &mut shutdown => None,
                    result = runtime.poll() => Some(result),
                };
                let Some(result) = poll_result else {
                    cancel_and_shutdown(&mut runtime, &status).await?;
                    return Ok(());
                };
                match result {
                    Ok(WorktreeHostedRuntimePoll::Idle
                        | WorktreeHostedRuntimePoll::Automatic(_)
                        | WorktreeHostedRuntimePoll::Manual(_)) => {
                        publish_status(&status, runtime.status()).await;
                    }
                    Err(_) => {
                        *status.write().await = ServerWorktreeHostStatus::failed_from(runtime.status());
                        return Err(ServerWorktreeHostError::RuntimeFailed);
                    }
                }
            }
        }
    }
}

async fn cancel_and_shutdown<C, W, X>(
    runtime: &mut WorktreeHostedRuntime<C, W, X>,
    status: &RwLock<ServerWorktreeHostStatus>,
) -> Result<(), ServerWorktreeHostError>
where
    C: WorktreeRuntimeClock,
    W: WorktreeWatcher,
    X: WorktreeRuntimeCycle,
{
    let _ = runtime.request_cancel();
    if runtime.shutdown().is_err() {
        *status.write().await = ServerWorktreeHostStatus::failed_from(runtime.status());
        return Err(ServerWorktreeHostError::RuntimeFailed);
    }
    publish_status(status, runtime.status()).await;
    Ok(())
}

async fn publish_status(status: &RwLock<ServerWorktreeHostStatus>, runtime: WorktreeRuntimeStatus) {
    *status.write().await = ServerWorktreeHostStatus::from_runtime(runtime);
}

fn fingerprint_root(root: &std::path::Path) -> Sha256 {
    let mut hasher = Sha256Hasher::new();
    hasher.update(b"haze-sync/worktree-root/v1\0");
    hasher.update(root.to_string_lossy().as_bytes());
    Sha256::from_bytes(hasher.finalize().into())
}

#[derive(Clone, Copy, Debug)]
struct TokioWorktreeClock {
    origin: time::Instant,
}

impl TokioWorktreeClock {
    fn new() -> Self {
        Self {
            origin: time::Instant::now(),
        }
    }
}

impl WorktreeRuntimeClock for TokioWorktreeClock {
    fn now(&self) -> Duration {
        time::Instant::now().duration_since(self.origin)
    }
}

#[cfg(test)]
#[path = "worktree_host_tests.rs"]
mod tests;
