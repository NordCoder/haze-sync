use crate::*;
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

#[derive(Debug, Clone)]
struct FakeClock {
    now: Rc<Cell<Duration>>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            now: Rc::new(Cell::new(Duration::ZERO)),
        }
    }

    fn advance(&self, duration: Duration) {
        self.now.set(self.now.get().checked_add(duration).unwrap());
    }
}

impl WorktreeRuntimeClock for FakeClock {
    fn now(&self) -> Duration {
        self.now.get()
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
}

impl WorktreeRuntimeCycle for FakeCycle {
    fn run_cycle(
        &mut self,
        request: WorktreeRuntimeCycleRequest,
    ) -> Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure> {
        self.requests.push(request);
        self.responses.pop_front().unwrap_or_else(|| {
            Ok(WorktreeRuntimeCycleSummary {
                full_scan_completed: request.full_scan_required,
                ..WorktreeRuntimeCycleSummary::default()
            })
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

#[test]
fn duplicate_and_reordered_hints_debounce_into_one_full_scan_cycle() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::Bidirectional, clock.clone());
    let start = runtime.start().unwrap();
    assert_eq!(start.watcher, WorktreeRuntimeWatcherState::Running);

    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Startup,
            ..
        }
    ));

    runtime.watcher_mut().push_hint(8);
    runtime.watcher_mut().push_hint(3);
    runtime.watcher_mut().push_hint(8);
    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    assert_eq!(runtime.status().pending_watcher_hints, 3);

    clock.advance(Duration::from_millis(49));
    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    clock.advance(Duration::from_millis(1));
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::WatcherHint,
            ..
        }
    ));

    let requests = &runtime.executor().requests;
    assert_eq!(requests.len(), 2);
    let watcher_request = requests[1];
    assert!(watcher_request.full_scan_required);
    assert!(watcher_request.import_enabled);
    assert!(watcher_request.export_enabled);
    assert_eq!(watcher_request.coalesced_watcher_hints, 3);
    assert_eq!(runtime.status().cycles_completed, 2);
    assert!(!runtime.status().cycle_in_progress);
}

#[test]
fn periodic_cycle_recovers_when_watcher_delivers_no_hint() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock.clone());
    runtime.start().unwrap();
    runtime.poll().unwrap();

    clock.advance(Duration::from_secs(9));
    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    clock.advance(Duration::from_secs(1));
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Periodic,
            ..
        }
    ));

    let request = runtime.executor().requests[1];
    assert!(request.full_scan_required);
    assert!(request.import_enabled);
    assert!(!request.export_enabled);
    assert_eq!(request.coalesced_watcher_hints, 0);
}

#[test]
fn watcher_hint_consumption_is_bounded_per_host_poll() {
    let clock = FakeClock::new();
    let bounded_policy = WorktreeRuntimePolicy::new(
        Duration::from_millis(10),
        Duration::from_secs(10),
        2,
        WorktreeRuntimeCycleBudget {
            max_import_actions: 1,
            max_delete_candidates: 1,
            max_export_actions: 1,
        },
    )
    .unwrap();
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        bounded_policy,
        clock.clone(),
        FakeWatcher::default(),
        FakeCycle::default(),
    );
    runtime.start().unwrap();
    runtime.poll().unwrap();
    runtime.watcher_mut().push_hint(3);
    runtime.watcher_mut().push_hint(2);
    runtime.watcher_mut().push_hint(1);

    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    assert_eq!(runtime.status().pending_watcher_hints, 2);
    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    assert_eq!(runtime.status().pending_watcher_hints, 3);
    clock.advance(Duration::from_millis(10));
    runtime.poll().unwrap();

    assert_eq!(runtime.executor().requests.len(), 2);
    assert_eq!(runtime.executor().requests[1].coalesced_watcher_hints, 3);
}

#[test]
fn cancellation_prevents_startup_cycle_and_shutdown_is_explicit() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock);
    runtime.start().unwrap();
    runtime.request_cancel().unwrap();

    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Cancelled);
    assert!(runtime.executor().requests.is_empty());
    let shutdown = runtime.shutdown().unwrap();
    assert_eq!(shutdown.watcher, WorktreeRuntimeWatcherState::Stopped);
    assert_eq!(runtime.watcher_mut().shutdowns, 1);
    assert_eq!(
        runtime.status().lifecycle,
        WorktreeRuntimeLifecycle::Shutdown
    );
    assert_eq!(
        runtime.poll().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyShutdown
    );
}

#[test]
fn adapter_modes_produce_distinct_cycle_capabilities() {
    let cases = [
        (WorktreeMode::Disabled, false, false, false, false),
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

        if mode == WorktreeMode::Disabled {
            assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Disabled);
            assert!(runtime.executor().requests.is_empty());
            continue;
        }

        runtime.poll().unwrap();
        let request = runtime.executor().requests[0];
        assert_eq!(request.full_scan_required, full_scan);
        assert_eq!(request.import_enabled, import_enabled);
        assert_eq!(request.export_enabled, export_enabled);
    }
}

#[test]
fn watcher_failure_degrades_to_periodic_full_scans() {
    let clock = FakeClock::new();
    let watcher = FakeWatcher {
        start_failure: Some(WorktreeWatcherFailure::Start),
        ..FakeWatcher::default()
    };
    let mut runtime = WorktreeRuntimeService::new(
        WorktreeMode::ImportOnly,
        policy(),
        clock.clone(),
        watcher,
        FakeCycle::default(),
    );

    let start = runtime.start().unwrap();
    assert_eq!(
        start.watcher,
        WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Start)
    );
    runtime.poll().unwrap();
    clock.advance(Duration::from_secs(10));
    runtime.poll().unwrap();

    assert_eq!(runtime.executor().requests.len(), 2);
    assert!(runtime
        .executor()
        .requests
        .iter()
        .all(|request| request.full_scan_required));
}

#[test]
fn executor_contract_rejects_missing_scan_mode_work_and_budget_overflow() {
    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ImportOnly, clock.clone());
    runtime
        .executor_mut()
        .responses
        .push_back(Ok(WorktreeRuntimeCycleSummary {
            full_scan_completed: false,
            ..WorktreeRuntimeCycleSummary::default()
        }));
    runtime.start().unwrap();
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::RequiredScanMissing,
            ..
        }
    ));
    assert!(!runtime.status().startup_cycle_pending);
    assert_eq!(runtime.poll().unwrap(), WorktreeRuntimePoll::Idle);
    clock.advance(Duration::from_secs(10));
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleCompleted {
            cause: WorktreeRuntimeCycleCause::Periodic,
            ..
        }
    ));

    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::ExportOnly, clock);
    runtime
        .executor_mut()
        .push_summary(WorktreeRuntimeCycleSummary {
            planned_imports: 1,
            ..WorktreeRuntimeCycleSummary::default()
        });
    runtime.start().unwrap();
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::ImportModeViolation,
            ..
        }
    ));

    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::Bidirectional, clock);
    runtime
        .executor_mut()
        .push_summary(WorktreeRuntimeCycleSummary {
            full_scan_completed: true,
            planned_imports: 5,
            ..WorktreeRuntimeCycleSummary::default()
        });
    runtime.start().unwrap();
    assert!(matches!(
        runtime.poll().unwrap(),
        WorktreeRuntimePoll::CycleRejected {
            violation: WorktreeRuntimeContractViolation::BudgetExceeded,
            ..
        }
    ));
}

#[test]
fn lifecycle_rejects_implicit_restart_and_invalid_policy() {
    assert_eq!(
        WorktreeRuntimePolicy::new(
            Duration::ZERO,
            Duration::ZERO,
            1,
            WorktreeRuntimeCycleBudget {
                max_import_actions: 1,
                max_delete_candidates: 1,
                max_export_actions: 1,
            },
        )
        .unwrap_err(),
        WorktreeRuntimePolicyError::ZeroPeriodicInterval
    );

    let clock = FakeClock::new();
    let mut runtime = service(WorktreeMode::Disabled, clock);
    assert_eq!(
        runtime.poll().unwrap_err(),
        WorktreeRuntimeLifecycleError::NotStarted
    );
    runtime.start().unwrap();
    assert_eq!(
        runtime.start().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyStarted
    );
    runtime.shutdown().unwrap();
    assert_eq!(
        runtime.start().unwrap_err(),
        WorktreeRuntimeLifecycleError::AlreadyShutdown
    );
}
