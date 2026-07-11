use crate::*;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

#[derive(Debug, Clone)]
struct FakeClock {
    now: Arc<Mutex<Duration>>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            now: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().unwrap();
        *now = now.checked_add(duration).unwrap();
    }
}

impl WorktreeRuntimeClock for FakeClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

#[derive(Debug, Default)]
struct FakeWatcher {
    start_failure: Option<WorktreeWatcherFailure>,
    shutdown_failure: Option<WorktreeWatcherFailure>,
    polls: VecDeque<Result<WorktreeWatcherPoll, WorktreeWatcherFailure>>,
    starts: usize,
    shutdowns: usize,
}

impl FakeWatcher {
    fn push_hint(&mut self, sequence: u64) {
        self.polls
            .push_back(Ok(WorktreeWatcherPoll::Hint(WorktreeWatcherHint {
                sequence,
            })));
    }
}

impl WorktreeWatcher for FakeWatcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> {
        self.starts += 1;
        self.start_failure.map_or(Ok(()), Err)
    }

    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> {
        self.polls
            .pop_front()
            .unwrap_or(Ok(WorktreeWatcherPoll::Idle))
    }

    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> {
        self.shutdowns += 1;
        self.shutdown_failure.map_or(Ok(()), Err)
    }
}

#[derive(Debug, Default)]
struct FakeCycle {
    requests: Vec<WorktreeRuntimeCycleRequest>,
    responses: VecDeque<Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>>,
}

impl FakeCycle {
    fn push_summary(&mut self, summary: WorktreeRuntimeCycleSummary) {
        self.responses.push_back(Ok(summary));
    }

    fn push_failure(&mut self, failure: WorktreeRuntimeCycleFailure) {
        self.responses.push_back(Err(failure));
    }
}

impl WorktreeRuntimeCycle for FakeCycle {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        self.requests.push(request);
        let response = self.responses.pop_front().unwrap_or_else(|| {
            Ok(WorktreeRuntimeCycleSummary {
                full_scan_completed: request.full_scan_required,
                ..WorktreeRuntimeCycleSummary::default()
            })
        });
        Box::pin(async move {
            if cancellation.is_cancelled() {
                Err(WorktreeRuntimeCycleFailure::Cancelled)
            } else {
                response
            }
        })
    }
}

#[derive(Debug, Default)]
struct ControlledState {
    ready: AtomicBool,
    active: AtomicUsize,
    max_active: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

#[derive(Debug, Clone)]
struct ControlledHandle(Arc<ControlledState>);

impl ControlledHandle {
    fn complete(&self) {
        self.0.ready.store(true, Ordering::Release);
        if let Some(waker) = self.0.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    fn active(&self) -> usize {
        self.0.active.load(Ordering::Acquire)
    }

    fn max_active(&self) -> usize {
        self.0.max_active.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
struct ControlledCycle {
    state: Arc<ControlledState>,
    requests: Arc<Mutex<Vec<WorktreeRuntimeCycleRequest>>>,
    summary: WorktreeRuntimeCycleSummary,
}

impl ControlledCycle {
    fn new(summary: WorktreeRuntimeCycleSummary) -> (Self, ControlledHandle) {
        let state = Arc::new(ControlledState::default());
        (
            Self {
                state: state.clone(),
                requests: Arc::new(Mutex::new(Vec::new())),
                summary,
            },
            ControlledHandle(state),
        )
    }
}

struct ControlledFuture {
    state: Arc<ControlledState>,
    cancellation: WorktreeCancellationToken,
    summary: WorktreeRuntimeCycleSummary,
    finished: bool,
}

impl Future for ControlledFuture {
    type Output = Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancellation.is_cancelled() {
            self.finish();
            return Poll::Ready(Err(WorktreeRuntimeCycleFailure::Cancelled));
        }
        if self.state.ready.load(Ordering::Acquire) {
            let summary = self.summary;
            self.finish();
            return Poll::Ready(Ok(summary));
        }
        *self.state.waker.lock().unwrap() = Some(context.waker().clone());
        Poll::Pending
    }
}

impl ControlledFuture {
    fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            self.state.active.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

impl Drop for ControlledFuture {
    fn drop(&mut self) {
        self.finish();
    }
}

impl WorktreeRuntimeCycle for ControlledCycle {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        self.requests.lock().unwrap().push(request);
        let active = self.state.active.fetch_add(1, Ordering::AcqRel) + 1;
        self.state.max_active.fetch_max(active, Ordering::AcqRel);
        Box::pin(ControlledFuture {
            state: self.state.clone(),
            cancellation,
            summary: self.summary,
            finished: false,
        })
    }
}

fn policy() -> WorktreeRuntimePolicy {
    WorktreeRuntimePolicy::new(
        Duration::from_millis(50),
        Duration::from_secs(10),
        8,
        WorktreeRuntimeCycleBudget {
            max_import_actions: 4,
            max_delete_candidates: 3,
            max_export_actions: 5,
        },
    )
    .unwrap()
}

fn service(
    mode: WorktreeMode,
    clock: FakeClock,
) -> WorktreeRuntimeService<FakeClock, FakeWatcher, FakeCycle> {
    WorktreeRuntimeService::new(
        mode,
        policy(),
        clock,
        FakeWatcher::default(),
        FakeCycle::default(),
    )
}

#[derive(Debug)]
struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    future.poll(&mut context)
}

fn assert_send<T: Send>(_: &T) {}

#[test]
fn async_executor_is_awaited_and_real_summary_or_failure_is_returned() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::Bidirectional, clock);
    let summary = WorktreeRuntimeCycleSummary {
        full_scan_completed: true,
        scanned_files: 7,
        planned_imports: 2,
        submitted_imports: 2,
        applied_exports: 1,
        ..WorktreeRuntimeCycleSummary::default()
    };
    runtime.executor_mut().push_summary(summary);
    runtime.start().unwrap();

    let future = runtime.poll();
    assert_send(&future);
    assert_eq!(
        block_on(future).unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Startup,
            summary,
        }
    );

    runtime.executor_mut().push_failure(WorktreeRuntimeCycleFailure::Export);
    let clock = runtime.cancellation_token();
    assert!(!clock.is_cancelled());
    runtime.request_cancel().unwrap();
    assert_eq!(block_on(runtime.poll()).unwrap(), WorktreeRuntimePoll::Cancelled);
}

#[test]
fn duplicate_and_reordered_hints_debounce_into_one_full_scan_cycle() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::Bidirectional, clock.clone());
    runtime.start().unwrap();
    block_on(runtime.poll()).unwrap();

    runtime.watcher_mut().push_hint(8);
    runtime.watcher_mut().push_hint(3);
    runtime.watcher_mut().push_hint(8);
    assert_eq!(block_on(runtime.poll()).unwrap(), WorktreeRuntimePoll::Idle);
    assert_eq!(runtime.status().pending_watcher_hints, 3);
    clock.advance(Duration::from_millis(50));
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::WatcherHint,
            ..
        }
    ));

    let request = runtime.executor().requests[1];
    assert!(request.full_scan_required);
    assert!(request.import_enabled);
    assert!(request.export_enabled);
    assert_eq!(request.coalesced_watcher_hints, 3);
}

#[test]
fn periodic_cycle_recovers_without_watcher_delivery() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock.clone());
    runtime.start().unwrap();
    block_on(runtime.poll()).unwrap();

    clock.advance(Duration::from_secs(10));
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Periodic,
            ..
        }
    ));
    let request = runtime.executor().requests[1];
    assert!(request.full_scan_required);
    assert!(request.import_enabled);
    assert!(!request.export_enabled);
}

#[test]
fn poll_awaits_one_cycle_and_never_overlaps_executor_calls() {
    let clock = FakeClock::new();
    let summary = WorktreeRuntimeCycleSummary {
        full_scan_completed: true,
        ..WorktreeRuntimeCycleSummary::default()
    };
    let (cycle, handle) = ControlledCycle::new(summary);
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy(),
        clock,
        FakeWatcher::default(),
        cycle,
    );
    runtime.start().unwrap();

    let mut future = Box::pin(runtime.poll());
    assert!(poll_once(future.as_mut()).is_pending());
    assert_eq!(handle.active(), 1);
    assert_eq!(handle.max_active(), 1);
    handle.complete();
    assert_eq!(
        block_on(future).unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Startup,
            summary,
        }
    );
    assert_eq!(handle.active(), 0);
    assert_eq!(handle.max_active(), 1);
    assert_eq!(block_on(runtime.poll()).unwrap(), WorktreeRuntimePoll::Idle);
}

#[test]
fn cancellation_before_due_cycle_prevents_execution() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock);
    runtime.start().unwrap();
    runtime.request_cancel().unwrap();

    assert_eq!(block_on(runtime.poll()).unwrap(), WorktreeRuntimePoll::Cancelled);
    assert!(runtime.executor().requests.is_empty());
}

#[test]
fn in_flight_cancellation_maps_safely_and_prevents_later_cycles() {
    let clock = FakeClock::new();
    let (cycle, handle) = ControlledCycle::new(WorktreeRuntimeCycleSummary {
        full_scan_completed: true,
        ..WorktreeRuntimeCycleSummary::default()
    });
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy(),
        clock,
        FakeWatcher::default(),
        cycle,
    );
    runtime.start().unwrap();
    let cancellation = runtime.cancellation_token();

    let mut future = Box::pin(runtime.poll());
    assert!(poll_once(future.as_mut()).is_pending());
    assert_eq!(handle.active(), 1);
    cancellation.cancel();
    handle.complete();
    assert_eq!(
        block_on(future).unwrap(),
        WorktreeRuntimePoll::CycleFailed {
            cause: WorktreeRuntimeCycleCause::Startup,
            failure: WorktreeRuntimeCycleFailure::Cancelled,
        }
    );
    assert_eq!(handle.active(), 0);
    assert_eq!(runtime.status().lifecycle, WorktreeRuntimeLifecycle::Cancelling);
    assert_eq!(block_on(runtime.poll()).unwrap(), WorktreeRuntimePoll::Cancelled);
}

#[test]
fn modes_include_explicit_inert_dry_run() {
    let cases = [
        (WorktreeMode::ReadOnly, false, false, false, true),
        (WorktreeMode::ImportOnly, true, true, true, false),
        (WorktreeMode::ExportOnly, false, false, false, true),
        (WorktreeMode::Bidirectional, true, true, true, true),
    ];
    for (mode, watcher_started, full_scan, import_enabled, export_enabled) in cases {
        let clock = FakeClock::new();
        let mut runtime = service(mode, clock);
        runtime.start().unwrap();
        assert_eq!(runtime.watcher_mut().starts > 0, watcher_started);
        block_on(runtime.poll()).unwrap();
        let request = runtime.executor().requests[0];
        assert_eq!(request.full_scan_required, full_scan);
        assert_eq!(request.import_enabled, import_enabled);
        assert_eq!(request.export_enabled, export_enabled);
    }

    let mut disabled = service(WorktreeMode::Disabled, FakeClock::new());
    assert!(!disabled.start().unwrap().startup_cycle_pending);
    assert_eq!(block_on(disabled.poll()).unwrap(), WorktreeRuntimePoll::Disabled);
    assert!(disabled.executor().requests.is_empty());

    let mut dry_run = service(WorktreeMode::DryRun, FakeClock::new());
    assert!(!dry_run.start().unwrap().startup_cycle_pending);
    assert_eq!(block_on(dry_run.poll()).unwrap(), WorktreeRuntimePoll::DryRun);
    assert!(dry_run.executor().requests.is_empty());
    assert!(WorktreeMode::DryRun.allows_manual_planning());
    assert!(!WorktreeMode::DryRun.permits_mutation());
    assert!(!WorktreeMode::DryRun.observes_local());
    assert!(!WorktreeMode::DryRun.imports_local());
    assert!(!WorktreeMode::DryRun.exports_core());
    let request = WorktreeRuntimeCycleRequest::manual_dry_run(policy().budget());
    assert_eq!(request.mode, WorktreeMode::DryRun);
    assert!(request.full_scan_required);
    assert!(!request.import_enabled);
    assert!(!request.export_enabled);
}

#[test]
fn watcher_close_and_failure_keep_periodic_correctness_and_cleanup() {
    let clock = FakeClock::new();
    let mut watcher = FakeWatcher::default();
    watcher.polls.push_back(Ok(WorktreeWatcherPoll::Closed));
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy(),
        clock.clone(),
        watcher,
        FakeCycle::default(),
    );
    runtime.start().unwrap();
    block_on(runtime.poll()).unwrap();
    assert_eq!(runtime.status().watcher, WorktreeRuntimeWatcherState::Closed);
    clock.advance(Duration::from_secs(10));
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Periodic,
            ..
        }
    ));
    assert_eq!(runtime.shutdown().unwrap().watcher, WorktreeRuntimeWatcherState::Stopped);
    assert_eq!(runtime.watcher_mut().shutdowns, 1);

    let clock = FakeClock::new();
    let mut watcher = FakeWatcher::default();
    watcher.polls.push_back(Err(WorktreeWatcherFailure::Shutdown));
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy(),
        clock,
        watcher,
        FakeCycle::default(),
    );
    runtime.start().unwrap();
    block_on(runtime.poll()).unwrap();
    assert_eq!(
        runtime.status().watcher,
        WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Poll)
    );
}

#[test]
fn summary_contract_validation_and_budgets_remain_exact() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock.clone());
    runtime.executor_mut().push_summary(WorktreeRuntimeCycleSummary::default());
    runtime.start().unwrap();
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::RequiredScanMissing,
            ..
        }
    ));

    clock.advance(Duration::from_secs(10));
    runtime.executor_mut().push_summary(WorktreeRuntimeCycleSummary {
        full_scan_completed: true,
        planned_imports: 5,
        ..WorktreeRuntimeCycleSummary::default()
    });
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::BudgetExceeded,
            ..
        }
    ));

    let mut export = service(WorktreeMode::ExportOnly, FakeClock::new());
    export.executor_mut().push_summary(WorktreeRuntimeCycleSummary {
        planned_imports: 1,
        ..WorktreeRuntimeCycleSummary::default()
    });
    export.start().unwrap();
    assert!(matches!(
        block_on(export.poll()).unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::ImportModeViolation,
            ..
        }
    ));
}

#[test]
fn lifecycle_protections_and_safe_debug_output_remain_explicit() {
    let mut runtime = service(WorktreeMode::Disabled, FakeClock::new());
    assert_eq!(
        runtime.shutdown().unwrap_err(),
        WorktreeRuntimeLifecycleError::NotStarted
    );
    assert_eq!(
        block_on(runtime.poll()).unwrap_err(),
        WorktreeRuntimeLifecycleError::NotStarted
    );
    runtime.start().unwrap();
    assert_eq!(
        runtime.start().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyStarted
    );
    let debug = format!("{runtime:?}");
    assert!(debug.contains("WorktreeRuntimeService"));
    assert!(!debug.contains("executor"));
    assert!(!debug.contains("watcher:"));
    assert!(!debug.contains('/'));
    runtime.shutdown().unwrap();
    assert_eq!(
        runtime.shutdown().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyShutdown
    );
    assert_eq!(
        runtime.start().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyShutdown
    );
}
