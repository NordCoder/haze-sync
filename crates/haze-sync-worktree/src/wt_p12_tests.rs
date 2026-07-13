use crate::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
struct Clock;

impl WorktreeRuntimeClock for Clock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

#[derive(Debug, Default)]
struct Watcher;

impl WorktreeWatcher for Watcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure> {
        Ok(())
    }

    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure> {
        Ok(WorktreeWatcherPoll::Idle)
    }

    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure> {
        Ok(())
    }
}

#[derive(Debug)]
struct ControlledCycle {
    ready: Arc<AtomicBool>,
}

impl WorktreeRuntimeCycle for ControlledCycle {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        Box::pin(ControlledFuture {
            ready: self.ready.clone(),
            request,
            cancellation,
        })
    }
}

struct ControlledFuture {
    ready: Arc<AtomicBool>,
    request: WorktreeRuntimeCycleRequest,
    cancellation: WorktreeCancellationToken,
}

impl Future for ControlledFuture {
    type Output = Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancellation.is_cancelled() {
            Poll::Ready(Err(WorktreeRuntimeCycleFailure::Cancelled))
        } else if self.ready.load(Ordering::Acquire) {
            Poll::Ready(Ok(WorktreeRuntimeCycleSummary {
                full_scan_completed: self.request.full_scan_required,
                ..Default::default()
            }))
        } else {
            Poll::Pending
        }
    }
}

struct Noop;

impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    future.poll(&mut context)
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(output) = poll_once(future.as_mut()) {
            return output;
        }
        std::thread::yield_now();
    }
}

fn budget() -> WorktreeRuntimeCycleBudget {
    WorktreeRuntimeCycleBudget {
        max_import_actions: 1,
        max_delete_candidates: 1,
        max_export_actions: 1,
    }
}

fn hosted() -> (
    WorktreeHostedRuntime<Clock, Watcher, ControlledCycle>,
    WorktreeRuntimeManualHandle,
    Arc<AtomicBool>,
) {
    let ready = Arc::new(AtomicBool::new(false));
    let policy = WorktreeRuntimePolicy::new(
        Duration::ZERO,
        Duration::from_secs(60),
        1,
        budget(),
    )
    .unwrap();
    let service = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy,
        Clock,
        Watcher,
        ControlledCycle {
            ready: ready.clone(),
        },
    );
    let (runtime, handle) = WorktreeHostedRuntime::new(service, 1).unwrap();
    (runtime, handle, ready)
}

#[test]
fn passive_status_tracks_created_running_busy_cancelling_and_shutdown() {
    let (mut runtime, handle, ready) = hosted();
    let status = handle.status_handle();
    assert_eq!(
        status.status(),
        WorktreeRuntimeManualStatus {
            lifecycle: WorktreeRuntimeLifecycle::Created,
            busy: false,
        }
    );

    runtime.start().unwrap();
    assert_eq!(
        status.status(),
        WorktreeRuntimeManualStatus {
            lifecycle: WorktreeRuntimeLifecycle::Running,
            busy: false,
        }
    );

    let ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    assert!(status.status().busy);
    assert!(matches!(
        handle.submit(WorktreeRuntimeManualRequest::new(true, budget())),
        WorktreeRuntimeManualSubmission::Busy
    ));
    assert!(status.status().busy);

    let mut poll = Box::pin(runtime.poll());
    assert!(poll_once(poll.as_mut()).is_pending());
    assert!(status.status().busy);
    ready.store(true, Ordering::Release);
    assert!(matches!(
        block_on(poll).unwrap(),
        WorktreeHostedRuntimePoll::Manual(WorktreeRuntimeManualOutcome::Completed(_))
    ));
    assert!(!status.status().busy);
    assert!(matches!(
        ticket.try_result(),
        WorktreeRuntimeManualTicketPoll::Completed(WorktreeRuntimeManualOutcome::Completed(_))
    ));

    runtime.request_cancel().unwrap();
    assert_eq!(status.status().lifecycle, WorktreeRuntimeLifecycle::Cancelling);
    assert!(!status.status().busy);
    runtime.shutdown().unwrap();
    assert_eq!(status.status().lifecycle, WorktreeRuntimeLifecycle::Shutdown);
    assert!(!status.status().busy);
}

#[test]
fn cancellation_by_drop_clears_status_and_preserves_typed_ticket_result() {
    let (mut runtime, handle, _) = hosted();
    let status = runtime.manual_status_handle();
    runtime.start().unwrap();
    let ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    let mut poll = Box::pin(runtime.poll());
    assert!(poll_once(poll.as_mut()).is_pending());
    assert!(status.status().busy);
    drop(poll);
    assert!(!status.status().busy);
    assert_eq!(
        ticket.try_result(),
        WorktreeRuntimeManualTicketPoll::Completed(WorktreeRuntimeManualOutcome::Failed(
            WorktreeRuntimeCycleFailure::Cancelled
        ))
    );
}

#[test]
fn older_ticket_cannot_clear_newer_accepted_request_status() {
    let (mut runtime, handle, ready) = hosted();
    let status = handle.status_handle();
    runtime.start().unwrap();

    let old_ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    ready.store(true, Ordering::Release);
    block_on(runtime.poll()).unwrap();
    assert!(!status.status().busy);

    ready.store(false, Ordering::Release);
    let new_ticket = match handle.submit(WorktreeRuntimeManualRequest::new(true, budget())) {
        WorktreeRuntimeManualSubmission::Accepted(ticket) => ticket,
        other => panic!("unexpected submission: {other:?}"),
    };
    assert!(status.status().busy);
    drop(old_ticket);
    assert!(status.status().busy);
    drop(new_ticket);
    assert!(status.status().busy);

    let mut poll = Box::pin(runtime.poll());
    assert!(poll_once(poll.as_mut()).is_pending());
    drop(poll);
    assert!(!status.status().busy);
}

#[test]
fn passive_status_reads_are_side_effect_free_and_coarse() {
    let (runtime, handle, _) = hosted();
    let status = runtime.manual_status_handle();
    let first = status.status();
    let second = status.status();
    assert_eq!(first, second);
    assert_eq!(first.lifecycle, WorktreeRuntimeLifecycle::Created);
    assert!(!first.busy);
    let debug = format!("{status:?} {handle:?}");
    assert!(debug.contains("Created"));
    assert!(!debug.contains("request"));
    assert!(!debug.contains("path"));
    assert!(!debug.contains("token"));
}
