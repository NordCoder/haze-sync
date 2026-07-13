use super::*;
use haze_sync_worktree::{
    WorktreeCancellationToken, WorktreeRuntimeCycleCause, WorktreeRuntimeCycleFailure,
    WorktreeRuntimeCycleFuture, WorktreeRuntimeCycleSummary, WorktreeRuntimeManualOutcome,
    WorktreeRuntimeManualTicket, WorktreeRuntimeManualTicketPoll, WorktreeWatcherHint,
    WorktreeWatcherPoll,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    Mutex,
};
use tokio::sync::Notify;

#[derive(Clone)]
struct FakeClock(Arc<AtomicU64>);

impl FakeClock {
    fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    fn set_millis(&self, value: u64) {
        self.0.store(value, Ordering::Release);
    }
}

impl WorktreeRuntimeClock for FakeClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::Acquire))
    }
}

struct FakeWatcher {
    fail_start: bool,
    hint: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
    running: bool,
}

impl FakeWatcher {
    fn healthy() -> (Self, Arc<AtomicBool>, Arc<AtomicBool>) {
        let hint = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        (
            Self {
                fail_start: false,
                hint: hint.clone(),
                dropped: dropped.clone(),
                running: false,
            },
            hint,
            dropped,
        )
    }

    fn failing() -> (Self, Arc<AtomicBool>) {
        let dropped = Arc::new(AtomicBool::new(false));
        (
            Self {
                fail_start: true,
                hint: Arc::new(AtomicBool::new(false)),
                dropped: dropped.clone(),
                running: false,
            },
            dropped,
        )
    }
}

impl Drop for FakeWatcher {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

impl WorktreeWatcher for FakeWatcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> {
        if self.fail_start {
            return Err(WorktreeWatcherFailure::Start);
        }
        self.running = true;
        Ok(())
    }

    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> {
        if !self.running {
            return Err(WorktreeWatcherFailure::Poll);
        }
        if self.hint.swap(false, Ordering::AcqRel) {
            Ok(WorktreeWatcherPoll::Hint(WorktreeWatcherHint { sequence: 1 }))
        } else {
            Ok(WorktreeWatcherPoll::Idle)
        }
    }

    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> {
        self.running = false;
        Ok(())
    }
}

#[derive(Clone)]
struct ExecutorState {
    causes: Arc<Mutex<Vec<WorktreeRuntimeCycleCause>>>,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
    block: Arc<AtomicBool>,
    release: Arc<Notify>,
    cancelled: Arc<AtomicBool>,
}

impl ExecutorState {
    fn new() -> Self {
        Self {
            causes: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(AtomicUsize::new(0)),
            max_active: Arc::new(AtomicUsize::new(0)),
            block: Arc::new(AtomicBool::new(false)),
            release: Arc::new(Notify::new()),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn causes(&self) -> Vec<WorktreeRuntimeCycleCause> {
        self.causes.lock().expect("causes lock").clone()
    }
}

struct FakeExecutor(ExecutorState);

impl WorktreeRuntimeCycle for FakeExecutor {
    fn run_cycle<'a>(
        &'a mut self,
        request: haze_sync_worktree::WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        let state = self.0.clone();
        Box::pin(async move {
            state.causes.lock().expect("causes lock").push(request.cause);
            let active = state.active.fetch_add(1, Ordering::AcqRel) + 1;
            state.max_active.fetch_max(active, Ordering::AcqRel);
            while state.block.load(Ordering::Acquire) {
                tokio::select! {
                    _ = state.release.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(1)) => {
                        if cancellation.is_cancelled() {
                            state.cancelled.store(true, Ordering::Release);
                            state.active.fetch_sub(1, Ordering::AcqRel);
                            return Err(WorktreeRuntimeCycleFailure::Cancelled);
                        }
                    }
                }
            }
            state.active.fetch_sub(1, Ordering::AcqRel);
            Ok(WorktreeRuntimeCycleSummary {
                full_scan_completed: request.full_scan_required,
                ..WorktreeRuntimeCycleSummary::default()
            })
        })
    }
}

fn budget() -> WorktreeRuntimeCycleBudget {
    WorktreeRuntimeCycleBudget {
        max_import_actions: 1,
        max_delete_candidates: 1,
        max_export_actions: 1,
    }
}

fn config(mode: WorktreeMode, timeout: Duration) -> ServerWorktreeHostConfig {
    ServerWorktreeHostConfig::new(
        mode,
        Duration::from_millis(1),
        Duration::from_millis(10),
        4,
        budget(),
        timeout,
        1,
    )
    .expect("valid test config")
}

async fn test_host(
    mode: WorktreeMode,
    watcher: FakeWatcher,
    clock: FakeClock,
    executor: ExecutorState,
) -> Result<ServerWorktreeRuntimeHost, ServerWorktreeHostError> {
    let service = WorktreeRuntimeService::new(
        mode,
        config(mode, Duration::from_millis(100)).policy().unwrap(),
        clock,
        watcher,
        FakeExecutor(executor),
    );
    let (runtime, manual) = WorktreeHostedRuntime::new(service, 1).unwrap();
    ServerWorktreeRuntimeHost::start_runtime(
        runtime,
        manual,
        config(mode, Duration::from_millis(100)),
    )
    .await
}

async fn wait_for_cause(state: &ExecutorState, cause: WorktreeRuntimeCycleCause) {
    for _ in 0..100 {
        if state.causes().contains(&cause) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    panic!("expected cycle cause {cause:?}");
}

async fn wait_ticket(ticket: &WorktreeRuntimeManualTicket) -> WorktreeRuntimeManualOutcome {
    for _ in 0..100 {
        match ticket.try_result() {
            WorktreeRuntimeManualTicketPoll::Completed(outcome) => return outcome,
            WorktreeRuntimeManualTicketPoll::Pending => {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            WorktreeRuntimeManualTicketPoll::Closed => panic!("ticket closed"),
        }
    }
    panic!("ticket did not complete")
}

#[test]
fn defaults_are_disabled_and_bounded() {
    let config = ServerWorktreeHostConfig::from_adapter_mode(AdapterMode::Disabled).unwrap();
    assert_eq!(config.mode, WorktreeMode::Disabled);
    assert!(config.action_budget.max_import_actions <= MAX_ACTION_BUDGET);
    assert!(config.manual_capacity <= MAX_MANUAL_CAPACITY);
}

#[test]
fn invalid_bounds_fail_closed() {
    assert_eq!(
        ServerWorktreeHostConfig::new(
            WorktreeMode::Bidirectional,
            Duration::ZERO,
            Duration::ZERO,
            0,
            WorktreeRuntimeCycleBudget {
                max_import_actions: MAX_ACTION_BUDGET + 1,
                max_delete_candidates: 1,
                max_export_actions: 1,
            },
            Duration::ZERO,
            0,
        ),
        Err(ServerWorktreeHostError::InvalidConfig)
    );
}

#[tokio::test]
async fn disabled_host_is_inert_and_join_free() {
    let host = ServerWorktreeRuntimeHost::disabled(Duration::from_secs(1));
    assert_eq!(host.status().await.lifecycle, ServerWorktreeHostLifecycle::Disabled);
    assert_eq!(host.shutdown().await.unwrap().lifecycle, ServerWorktreeHostLifecycle::Disabled);
}

#[tokio::test]
async fn enabled_start_acknowledges_running_and_joins_shutdown() {
    let (watcher, _, dropped) = FakeWatcher::healthy();
    let host = test_host(WorktreeMode::ExportOnly, watcher, FakeClock::new(), ExecutorState::new())
        .await
        .unwrap();
    assert_eq!(host.status().await.lifecycle, ServerWorktreeHostLifecycle::Running);
    let status = host.shutdown().await.unwrap();
    assert_eq!(status.lifecycle, ServerWorktreeHostLifecycle::Shutdown);
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn startup_failure_is_propagated_and_task_is_cleaned_up() {
    let (watcher, dropped) = FakeWatcher::failing();
    let result = test_host(WorktreeMode::ExportOnly, watcher, FakeClock::new(), ExecutorState::new()).await;
    assert!(matches!(result, Err(ServerWorktreeHostError::RuntimeFailed)));
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn host_drives_startup_periodic_and_watcher_cycles_without_overlap() {
    let (watcher, hint, _) = FakeWatcher::healthy();
    let clock = FakeClock::new();
    let state = ExecutorState::new();
    let host = test_host(WorktreeMode::Bidirectional, watcher, clock.clone(), state.clone())
        .await
        .unwrap();
    wait_for_cause(&state, WorktreeRuntimeCycleCause::Startup).await;
    clock.set_millis(20);
    wait_for_cause(&state, WorktreeRuntimeCycleCause::Periodic).await;
    hint.store(true, Ordering::Release);
    clock.set_millis(30);
    wait_for_cause(&state, WorktreeRuntimeCycleCause::WatcherHint).await;
    assert_eq!(state.max_active.load(Ordering::Acquire), 1);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn manual_boundary_reports_busy_and_completes_dry_run() {
    let (watcher, _, _) = FakeWatcher::healthy();
    let state = ExecutorState::new();
    state.block.store(true, Ordering::Release);
    let host = test_host(WorktreeMode::DryRun, watcher, FakeClock::new(), state.clone())
        .await
        .unwrap();
    let first = host.submit_manual(WorktreeRuntimeManualRequest::dry_run(budget()));
    let ticket = match first {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected first submission: {other:?}"),
    };
    assert!(matches!(
        host.submit_manual(WorktreeRuntimeManualRequest::dry_run(budget())),
        WorktreeRuntimeManualSubmission::Busy
    ));
    state.block.store(false, Ordering::Release);
    state.release.notify_waiters();
    let outcome = wait_ticket(&ticket).await;
    assert!(matches!(outcome, WorktreeRuntimeManualOutcome::Completed(summary) if summary.full_scan_completed));
    assert_eq!(state.causes(), vec![WorktreeRuntimeCycleCause::Manual]);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_cancels_pending_cycle_and_mandatorily_joins() {
    let (watcher, _, dropped) = FakeWatcher::healthy();
    let state = ExecutorState::new();
    state.block.store(true, Ordering::Release);
    let host = test_host(WorktreeMode::ExportOnly, watcher, FakeClock::new(), state.clone())
        .await
        .unwrap();
    wait_for_cause(&state, WorktreeRuntimeCycleCause::Startup).await;
    let status = host.shutdown().await.unwrap();
    assert_eq!(status.lifecycle, ServerWorktreeHostLifecycle::Shutdown);
    assert!(state.cancelled.load(Ordering::Acquire));
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn bounded_shutdown_timeout_aborts_and_awaits_retained_task() {
    let status = Arc::new(RwLock::new(ServerWorktreeHostStatus::starting()));
    let join = tokio::spawn(async {
        std::future::pending::<()>().await;
        Ok(())
    });
    let host = ServerWorktreeRuntimeHost {
        manual: None,
        status: status.clone(),
        shutdown: None,
        join: Some(join),
        shutdown_timeout: Duration::from_millis(1),
    };
    assert_eq!(host.shutdown().await, Err(ServerWorktreeHostError::ShutdownTimedOut));
    assert_eq!(status.read().await.lifecycle, ServerWorktreeHostLifecycle::Failed);
}

#[test]
fn status_debug_and_errors_are_secret_safe() {
    let status = ServerWorktreeHostStatus::starting();
    let rendered = format!("{status:?} {}", ServerWorktreeHostError::BindingFailed);
    assert!(!rendered.contains("postgres://"));
    assert!(!rendered.contains("/srv/"));
    assert!(!rendered.contains("fingerprint"));
    assert!(!rendered.contains("payload"));
}
