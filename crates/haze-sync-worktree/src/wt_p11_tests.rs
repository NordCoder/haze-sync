use crate::*;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct Clock;
impl WorktreeRuntimeClock for Clock { fn now(&self) -> Duration { Duration::ZERO } }

#[derive(Debug, Default)]
struct Watcher;
impl WorktreeWatcher for Watcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> { Ok(()) }
    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> { Ok(WorktreeWatcherPoll::Idle) }
    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> { Ok(()) }
}

#[derive(Debug, Default)]
struct Cycle { requests: Vec<WorktreeRuntimeCycleRequest> }
impl WorktreeRuntimeCycle for Cycle {
    fn run_cycle<'a>(&'a mut self, request: WorktreeRuntimeCycleRequest, _: WorktreeCancellationToken) -> WorktreeRuntimeCycleFuture<'a> {
        self.requests.push(request);
        Box::pin(async move { Ok(WorktreeRuntimeCycleSummary { full_scan_completed: request.full_scan_required, ..Default::default() }) })
    }
}

struct Noop;
impl Wake for Noop { fn wake(self: Arc<Self>) {} }
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop { if let Poll::Ready(value) = future.as_mut().poll(&mut context) { return value; } std::thread::yield_now(); }
}
fn budget() -> WorktreeRuntimeCycleBudget { WorktreeRuntimeCycleBudget { max_import_actions: 2, max_delete_candidates: 2, max_export_actions: 2 } }
fn policy() -> WorktreeRuntimePolicy { WorktreeRuntimePolicy::new(Duration::ZERO, Duration::from_secs(60), 4, budget()).unwrap() }

#[test]
fn manual_cycle_is_scheduler_accounted() {
    let mut runtime = WorktreeRuntimeService::new(WorktreeMode::ImportOnly, policy(), Clock, Watcher, Cycle::default());
    runtime.start().unwrap();
    assert!(matches!(block_on(runtime.run_manual_cycle(WorktreeRuntimeManualRequest::new(true, budget()))), WorktreeRuntimeManualOutcome::Completed(_)));
    let status = runtime.status();
    assert_eq!(status.cycles_completed, 1);
    assert_eq!(status.last_cycle_cause, Some(WorktreeRuntimeCycleCause::Manual));
    assert!(!status.cycle_in_progress);
}

#[test]
fn manual_dry_run_is_full_scan_and_mutation_free() {
    let mut runtime = WorktreeRuntimeService::new(WorktreeMode::DryRun, policy(), Clock, Watcher, Cycle::default());
    runtime.start().unwrap();
    assert!(matches!(block_on(runtime.run_manual_cycle(WorktreeRuntimeManualRequest::dry_run(budget()))), WorktreeRuntimeManualOutcome::Completed(_)));
    let request = runtime.executor().requests[0];
    assert!(request.full_scan_required);
    assert!(!request.import_enabled);
    assert!(!request.export_enabled);
    assert_eq!(request.cause, WorktreeRuntimeCycleCause::Manual);
}

#[test]
fn manual_cycle_reports_lifecycle_states() {
    let mut runtime = WorktreeRuntimeService::new(WorktreeMode::ImportOnly, policy(), Clock, Watcher, Cycle::default());
    assert_eq!(block_on(runtime.run_manual_cycle(WorktreeRuntimeManualRequest::new(true, budget()))), WorktreeRuntimeManualOutcome::NotStarted);
    runtime.start().unwrap();
    runtime.request_cancel().unwrap();
    assert_eq!(block_on(runtime.run_manual_cycle(WorktreeRuntimeManualRequest::new(true, budget()))), WorktreeRuntimeManualOutcome::Cancelling);
    runtime.shutdown().unwrap();
    assert_eq!(block_on(runtime.run_manual_cycle(WorktreeRuntimeManualRequest::new(true, budget()))), WorktreeRuntimeManualOutcome::Shutdown);
}

#[test]
fn production_watcher_lifecycle_and_debug_are_path_free() {
    let root = unique_dir();
    std::fs::create_dir_all(&root).unwrap();
    let mut watcher = ProductionWorktreeWatcher::new(root.clone(), 2).unwrap();
    assert!(!format!("{watcher:?}").contains(root.to_string_lossy().as_ref()));
    watcher.start().unwrap();
    assert_eq!(watcher.state(), ProductionWorktreeWatcherState::Running);
    std::fs::write(root.join("note.md"), b"x").unwrap();
    let mut observed = false;
    for _ in 0..100 {
        match watcher.poll_hint().unwrap() {
            WorktreeWatcherPoll::Hint(hint) => { assert!(hint.sequence > 0); observed = true; break; }
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    assert!(observed);
    watcher.shutdown().unwrap();
    assert_eq!(watcher.state(), ProductionWorktreeWatcherState::Stopped);
    std::fs::remove_dir_all(root).unwrap();
}

fn unique_dir() -> PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("haze-worktree-watcher-{nonce}"))
}
