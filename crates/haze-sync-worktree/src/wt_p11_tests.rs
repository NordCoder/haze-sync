use crate::*;
use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
struct Clock(Arc<Mutex<Duration>>);

impl Clock {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Duration::ZERO)))
    }

    fn advance(&self, duration: Duration) {
        let mut now = self.0.lock().unwrap();
        *now = now.checked_add(duration).unwrap();
    }
}

impl WorktreeRuntimeClock for Clock {
    fn now(&self) -> Duration {
        *self.0.lock().unwrap()
    }
}

#[derive(Debug, Clone, Default)]
struct Watcher {
    polls: Arc<Mutex<VecDeque<Result<WorktreeWatcherPoll, WorktreeWatcherFailure>>>>,
}

impl Watcher {
    fn push_hint(&self, sequence: u64) {
        self.polls
            .lock()
            .unwrap()
            .push_back(Ok(WorktreeWatcherPoll::Hint(WorktreeWatcherHint {
                sequence,
            })));
    }
}

impl WorktreeWatcher for Watcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> {
        Ok(())
    }

    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> {
        self.polls
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(WorktreeWatcherPoll::Idle))
    }

    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> {
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Cycle {
    requests: Vec<WorktreeRuntimeCycleRequest>,
}

impl WorktreeRuntimeCycle for Cycle {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        self.requests.push(request);
        Box::pin(async move {
            if cancellation.is_cancelled() {
                Err(WorktreeRuntimeCycleFailure::Cancelled)
            } else {
                Ok(WorktreeRuntimeCycleSummary {
                    full_scan_completed: request.full_scan_required,
                    ..Default::default()
                })
            }
        })
    }
}

struct Noop;

impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        std::thread::yield_now();
    }
}

fn budget() -> WorktreeRuntimeCycleBudget {
    WorktreeRuntimeCycleBudget {
        max_import_actions: 2,
        max_delete_candidates: 2,
        max_export_actions: 2,
    }
}

fn policy() -> WorktreeRuntimePolicy {
    WorktreeRuntimePolicy::new(Duration::ZERO, Duration::from_secs(60), 4, budget()).unwrap()
}

fn hosted(
    mode: WorktreeMode,
    clock: Clock,
    watcher: Watcher,
) -> (
    WorktreeHostedRuntime<Clock, Watcher, Cycle>,
    WorktreeRuntimeManualHandle,
) {
    WorktreeHostedRuntime::new(
        WorktreeRuntimeService::new(mode, policy(), clock, watcher, Cycle::default()),
        1,
    )
    .unwrap()
}

#[test]
fn hosted_manual_request_has_reachable_busy_and_ticket_completion() {
    let (mut runtime, handle) = hosted(WorktreeMode::ImportOnly, Clock::new(), Watcher::default());
    assert!(matches!(
        handle.submit(WorktreeRuntimeManualRequest::new(true, budget())),
        WorktreeRuntimeManualSubmission::NotStarted
    ));
    runtime.start().unwrap();

    let ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    assert!(matches!(
        handle.submit(WorktreeRuntimeManualRequest::new(true, budget())),
        WorktreeRuntimeManualSubmission::Busy
    ));
    assert_eq!(ticket.try_result(), WorktreeRuntimeManualTicketPoll::Pending);
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeHostedRuntimePoll::Manual(WorktreeRuntimeManualOutcome::Completed(_))
    ));
    assert!(matches!(
        ticket.try_result(),
        WorktreeRuntimeManualTicketPoll::Completed(WorktreeRuntimeManualOutcome::Completed(_))
    ));
    assert_eq!(runtime.status().cycles_completed, 1);
    assert_eq!(
        runtime.status().last_cycle_cause,
        Some(WorktreeRuntimeCycleCause::Manual)
    );
}

#[test]
fn hosted_handle_reports_cancelling_and_shutdown() {
    let (mut runtime, handle) = hosted(WorktreeMode::ImportOnly, Clock::new(), Watcher::default());
    runtime.start().unwrap();
    runtime.request_cancel().unwrap();
    assert!(matches!(
        handle.submit(WorktreeRuntimeManualRequest::new(true, budget())),
        WorktreeRuntimeManualSubmission::Cancelling
    ));
    runtime.shutdown().unwrap();
    assert!(matches!(
        handle.submit(WorktreeRuntimeManualRequest::new(true, budget())),
        WorktreeRuntimeManualSubmission::Shutdown
    ));
}

#[test]
fn manual_cycle_preserves_startup_periodic_and_watcher_scheduling() {
    let clock = Clock::new();
    let watcher = Watcher::default();
    let watcher_handle = watcher.clone();
    let (mut runtime, handle) = hosted(WorktreeMode::ImportOnly, clock.clone(), watcher);
    runtime.start().unwrap();

    let _ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    block_on(runtime.poll()).unwrap();
    assert!(runtime.status().startup_cycle_pending);
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeHostedRuntimePoll::Automatic(WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Startup,
            ..
        })
    ));

    clock.advance(Duration::from_secs(60));
    let _ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    block_on(runtime.poll()).unwrap();
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeHostedRuntimePoll::Automatic(WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Periodic,
            ..
        })
    ));

    watcher_handle.push_hint(9);
    assert!(matches!(
        block_on(runtime.poll()).unwrap(),
        WorktreeHostedRuntimePoll::Automatic(WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::WatcherHint,
            ..
        })
    ));
}

#[test]
fn manual_dry_run_is_full_scan_and_mutation_free() {
    let (mut runtime, handle) = hosted(WorktreeMode::DryRun, Clock::new(), Watcher::default());
    runtime.start().unwrap();
    let ticket = match handle.submit(WorktreeRuntimeManualRequest::dry_run(budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    block_on(runtime.poll()).unwrap();
    assert!(matches!(
        ticket.try_result(),
        WorktreeRuntimeManualTicketPoll::Completed(WorktreeRuntimeManualOutcome::Completed(_))
    ));
}

#[test]
fn production_watcher_seam_is_bounded_coarse_and_path_free() {
    let root = unique_dir();
    std::fs::create_dir_all(&root).unwrap();
    let config = WorktreeConfig::new(root.clone()).unwrap();
    let mut watcher = ProductionWorktreeWatcher::new(&config, 1).unwrap();
    assert!(!format!("{watcher:?}").contains(root.to_string_lossy().as_ref()));
    watcher.start().unwrap();

    watcher.inject_test_event();
    watcher.inject_test_event();
    assert!(matches!(
        watcher.poll_hint().unwrap(),
        WorktreeWatcherPoll::Hint(WorktreeWatcherHint { sequence: 1 })
    ));
    assert!(matches!(
        watcher.poll_hint().unwrap(),
        WorktreeWatcherPoll::Hint(WorktreeWatcherHint { sequence: 2 })
    ));

    watcher.inject_test_failure();
    assert_eq!(watcher.poll_hint(), Err(WorktreeWatcherFailure::Poll));
    watcher.inject_test_closure();
    assert_eq!(watcher.poll_hint().unwrap(), WorktreeWatcherPoll::Closed);
    watcher.shutdown().unwrap();
    watcher.shutdown().unwrap();
    assert_eq!(watcher.state(), ProductionWorktreeWatcherState::Stopped);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_watcher_requires_validated_worktree_config() {
    assert!(WorktreeConfig::new(PathBuf::from("relative-root")).is_err());
    let root = unique_dir();
    let config = WorktreeConfig::new(root).unwrap();
    assert!(ProductionWorktreeWatcher::new(&config, 0).is_err());
}

fn unique_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("haze-worktree-watcher-{nonce}"))
}
