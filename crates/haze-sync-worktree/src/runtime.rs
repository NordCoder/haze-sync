//! Hostable Worktree runtime scheduling.
//!
//! The runtime is deliberately host-driven: it creates no unmanaged background
//! task and owns no Server startup wiring. Watcher events are path-free latency
//! hints only. Startup and periodic cycles still require authoritative full scans
//! whenever the selected mode observes local filesystem state.

use std::fmt;
use std::time::Duration;

/// Worktree adapter operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeMode {
    /// Runtime is inert: no watcher, scan, import, or export work.
    Disabled,
    /// Adapter may read Core/apply exports but must not write local facts to Core.
    ReadOnly,
    /// Observe and import local facts, but do not apply Core exports.
    ImportOnly,
    /// Apply Core exports, but do not watch or import local filesystem changes.
    ExportOnly,
    /// Observe/import local facts and apply Core exports.
    Bidirectional,
}

impl WorktreeMode {
    /// Whether any runtime cycle is enabled.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Whether this mode observes local state through authoritative scans.
    #[must_use]
    pub const fn observes_local(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    /// Whether local facts may be planned/submitted as imports.
    #[must_use]
    pub const fn imports_local(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    /// Whether authoritative Core changes may be materialized locally.
    #[must_use]
    pub const fn exports_core(self) -> bool {
        matches!(
            self,
            Self::ReadOnly | Self::ExportOnly | Self::Bidirectional
        )
    }
}

/// Monotonic clock used by the host-driven scheduler.
pub trait WorktreeRuntimeClock {
    /// Monotonic elapsed time from an arbitrary process-local origin.
    fn now(&self) -> Duration;
}

/// One path-free watcher hint.
///
/// `sequence` is diagnostic only. The runtime never relies on ordering or
/// uniqueness, so duplicate, missing, and reordered hints remain safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeWatcherHint {
    /// Optional watcher-local sequence value, ignored for correctness.
    pub sequence: u64,
}

/// Result of polling an optional filesystem watcher once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeWatcherPoll {
    /// One latency hint was available.
    Hint(WorktreeWatcherHint),
    /// No hint is currently available.
    Idle,
    /// Watcher stream ended; periodic scans continue to provide correctness.
    Closed,
}

/// Safe watcher failure category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeWatcherFailure {
    /// Watcher could not start.
    Start,
    /// Watcher polling failed.
    Poll,
    /// Watcher shutdown failed.
    Shutdown,
}

impl WorktreeWatcherFailure {
    /// Stable machine-readable category.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Start => "worktree_watcher_start_failed",
            Self::Poll => "worktree_watcher_poll_failed",
            Self::Shutdown => "worktree_watcher_shutdown_failed",
        }
    }
}

/// Optional watcher integration owned by a future host/composition layer.
///
/// The runtime normalizes an error to the operation that observed it, so public
/// status remains correct even if an implementation returns the wrong variant.
pub trait WorktreeWatcher {
    /// Start watcher resources.
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure>;

    /// Poll at most one path-free latency hint.
    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure>;

    /// Stop watcher resources.
    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure>;
}

/// Why an authoritative runtime cycle was scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeCycleCause {
    /// First cycle after explicit runtime startup.
    Startup,
    /// Debounced watcher hints requested lower-latency work.
    WatcherHint,
    /// Periodic correctness cycle, independent of watcher delivery.
    Periodic,
}

/// Bounded planning/submission budget communicated to the cycle executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleBudget {
    /// Maximum put/import actions the executor may plan in one cycle.
    pub max_import_actions: usize,
    /// Maximum guarded local-delete candidates the executor may plan in one cycle.
    pub max_delete_candidates: usize,
    /// Maximum Core export/materialization actions the executor may apply in one cycle.
    pub max_export_actions: usize,
}

/// Contract for one scan/import/export cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleRequest {
    /// Scheduling cause.
    pub cause: WorktreeRuntimeCycleCause,
    /// Active adapter mode.
    pub mode: WorktreeMode,
    /// A full scan must be completed before local facts are considered.
    pub full_scan_required: bool,
    /// Local import planning/submission is allowed.
    pub import_enabled: bool,
    /// Core export/materialization is allowed.
    pub export_enabled: bool,
    /// Per-cycle bounded work budget.
    pub budget: WorktreeRuntimeCycleBudget,
    /// Number of watcher hints coalesced into this cycle.
    pub coalesced_watcher_hints: usize,
}

/// Count-only result from one authoritative cycle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleSummary {
    /// Whether the required full scan completed.
    pub full_scan_completed: bool,
    /// Number of stable local files observed by the full scan.
    pub scanned_files: usize,
    /// Number of safely skipped filesystem entries.
    pub skipped_entries: usize,
    /// Number of put/import actions planned.
    pub planned_imports: usize,
    /// Number of guarded delete candidates planned.
    pub planned_deletes: usize,
    /// Number of accepted/submitted local import actions.
    pub submitted_imports: usize,
    /// Number of Core export/materialization actions applied.
    pub applied_exports: usize,
}

/// Safe cycle failure category supplied by the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeCycleFailure {
    /// Authoritative scan failed.
    Scan,
    /// Import/delete planning failed.
    Plan,
    /// Core/API submission failed.
    Submit,
    /// Core export/materialization failed.
    Export,
    /// Host cancelled the executor cooperatively.
    Cancelled,
}

impl WorktreeRuntimeCycleFailure {
    /// Stable machine-readable category.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Scan => "worktree_runtime_scan_failed",
            Self::Plan => "worktree_runtime_plan_failed",
            Self::Submit => "worktree_runtime_submit_failed",
            Self::Export => "worktree_runtime_export_failed",
            Self::Cancelled => "worktree_runtime_cycle_cancelled",
        }
    }
}

/// Host-provided authoritative cycle executor.
///
/// When `full_scan_required` is true, implementations must run the full scanner
/// before deriving local import/delete facts. Watcher hints must never be treated
/// as file facts themselves.
pub trait WorktreeRuntimeCycle {
    /// Run one bounded cycle synchronously.
    fn run_cycle(
        &mut self,
        request: WorktreeRuntimeCycleRequest,
    ) -> Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>;
}

/// Runtime scheduler policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimePolicy {
    debounce: Duration,
    periodic_cycle_interval: Duration,
    max_watcher_hints_per_poll: usize,
    budget: WorktreeRuntimeCycleBudget,
}

impl WorktreeRuntimePolicy {
    /// Construct a bounded scheduler policy.
    pub fn new(
        debounce: Duration,
        periodic_cycle_interval: Duration,
        max_watcher_hints_per_poll: usize,
        budget: WorktreeRuntimeCycleBudget,
    ) -> Result<Self, WorktreeRuntimePolicyError> {
        if periodic_cycle_interval.is_zero() {
            return Err(WorktreeRuntimePolicyError::ZeroPeriodicInterval);
        }
        if max_watcher_hints_per_poll == 0 {
            return Err(WorktreeRuntimePolicyError::ZeroHintBudget);
        }
        if budget.max_import_actions == 0
            || budget.max_delete_candidates == 0
            || budget.max_export_actions == 0
        {
            return Err(WorktreeRuntimePolicyError::ZeroCycleBudget);
        }
        Ok(Self {
            debounce,
            periodic_cycle_interval,
            max_watcher_hints_per_poll,
            budget,
        })
    }

    /// Debounce duration applied to watcher hints.
    #[must_use]
    pub const fn debounce(self) -> Duration {
        self.debounce
    }

    /// Maximum time between correctness cycles while enabled.
    #[must_use]
    pub const fn periodic_cycle_interval(self) -> Duration {
        self.periodic_cycle_interval
    }

    /// Maximum watcher entries consumed by one host poll.
    #[must_use]
    pub const fn max_watcher_hints_per_poll(self) -> usize {
        self.max_watcher_hints_per_poll
    }

    /// Bounded work budget for every cycle.
    #[must_use]
    pub const fn budget(self) -> WorktreeRuntimeCycleBudget {
        self.budget
    }
}

/// Invalid runtime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimePolicyError {
    /// Periodic correctness interval must be non-zero.
    ZeroPeriodicInterval,
    /// Host poll must be allowed to consume at least one watcher hint.
    ZeroHintBudget,
    /// Each action class must have a non-zero explicit cycle bound.
    ZeroCycleBudget,
}

impl fmt::Display for WorktreeRuntimePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroPeriodicInterval => "worktree periodic cycle interval must be non-zero",
            Self::ZeroHintBudget => "worktree watcher hint budget must be non-zero",
            Self::ZeroCycleBudget => "worktree runtime action budgets must be non-zero",
        })
    }
}

impl std::error::Error for WorktreeRuntimePolicyError {}

/// Explicit runtime lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLifecycle {
    /// Constructed but not started.
    Created,
    /// Started and eligible to run cycles.
    Running,
    /// Cancellation requested; no new cycle may start.
    Cancelling,
    /// Watcher resources were shut down and the runtime cannot restart.
    Shutdown,
}

/// Current optional watcher state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeWatcherState {
    /// Mode does not observe local filesystem state.
    Disabled,
    /// Watcher started successfully.
    Running,
    /// Watcher ended normally; periodic scans continue.
    Closed,
    /// Watcher failed; periodic scans continue.
    Failed(WorktreeWatcherFailure),
    /// Runtime shutdown completed.
    Stopped,
}

/// Last completed cycle outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLastCycle {
    /// Cycle completed and passed mode/budget validation.
    Completed(WorktreeRuntimeCycleSummary),
    /// Executor returned a safe failure category.
    Failed(WorktreeRuntimeCycleFailure),
    /// Executor violated its full-scan, mode, or budget contract.
    Rejected(WorktreeRuntimeContractViolation),
}

/// Safe scheduler status without local absolute paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeStatus {
    /// Explicit lifecycle state.
    pub lifecycle: WorktreeRuntimeLifecycle,
    /// Active adapter mode.
    pub mode: WorktreeMode,
    /// Optional watcher health.
    pub watcher: WorktreeRuntimeWatcherState,
    /// Whether a startup cycle is still pending.
    pub startup_cycle_pending: bool,
    /// Whether the synchronous executor is currently active.
    pub cycle_in_progress: bool,
    /// Number of coalesced watcher hints awaiting a cycle.
    pub pending_watcher_hints: usize,
    /// Total watcher hints observed since startup.
    pub watcher_hints_observed: u64,
    /// Total authoritative cycles completed successfully.
    pub cycles_completed: u64,
    /// Total executor failures or contract rejections.
    pub cycles_failed: u64,
    /// Cause of the most recently attempted cycle.
    pub last_cycle_cause: Option<WorktreeRuntimeCycleCause>,
    /// Count-only last cycle outcome.
    pub last_cycle: Option<WorktreeRuntimeLastCycle>,
}

/// Start result useful to a host/Server composition layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeStartSummary {
    /// Whether watcher startup succeeded.
    pub watcher: WorktreeRuntimeWatcherState,
    /// Whether the host should immediately poll a startup cycle.
    pub startup_cycle_pending: bool,
}

/// Shutdown result useful to a host/Server composition layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeShutdownSummary {
    /// Final watcher state. A failure is safe and count/path-free.
    pub watcher: WorktreeRuntimeWatcherState,
    /// Number of cycles completed before shutdown.
    pub cycles_completed: u64,
}

/// Result of one host-driven poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimePoll {
    /// No cycle was due.
    Idle,
    /// Runtime is disabled by mode.
    Disabled,
    /// Cancellation prevents new cycle work.
    Cancelled,
    /// One bounded cycle completed.
    CycleCompleted {
        /// Scheduling cause.
        cause: WorktreeRuntimeCycleCause,
        /// Count-only cycle result.
        summary: WorktreeRuntimeCycleSummary,
    },
    /// Executor returned a safe failure category.
    CycleFailed {
        /// Scheduling cause.
        cause: WorktreeRuntimeCycleCause,
        /// Safe failure category.
        failure: WorktreeRuntimeCycleFailure,
    },
    /// Executor violated the runtime request contract.
    CycleRejected {
        /// Scheduling cause.
        cause: WorktreeRuntimeCycleCause,
        /// Safe violation category.
        violation: WorktreeRuntimeContractViolation,
    },
}

/// Executor contract violation detected by the scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeContractViolation {
    /// Local-observing mode did not complete the required full scan.
    RequiredScanMissing,
    /// Executor planned or submitted local imports while mode forbids them.
    ImportModeViolation,
    /// Executor applied exports while mode forbids them.
    ExportModeViolation,
    /// Planned/submitted work exceeded the explicit cycle budget.
    BudgetExceeded,
    /// Submitted import count exceeded planned imports.
    InvalidImportCounts,
}

impl WorktreeRuntimeContractViolation {
    /// Stable machine-readable category.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RequiredScanMissing => "worktree_runtime_required_scan_missing",
            Self::ImportModeViolation => "worktree_runtime_import_mode_violation",
            Self::ExportModeViolation => "worktree_runtime_export_mode_violation",
            Self::BudgetExceeded => "worktree_runtime_cycle_budget_exceeded",
            Self::InvalidImportCounts => "worktree_runtime_invalid_import_counts",
        }
    }
}

/// Lifecycle misuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLifecycleError {
    /// Runtime was already started or is cancelling.
    AlreadyStarted,
    /// Runtime was already shut down and cannot restart.
    AlreadyShutdown,
    /// Runtime must be started before polling or cancellation.
    NotStarted,
}

impl fmt::Display for WorktreeRuntimeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyStarted => "worktree runtime is already started",
            Self::AlreadyShutdown => "worktree runtime is already shut down",
            Self::NotStarted => "worktree runtime is not started",
        })
    }
}

impl std::error::Error for WorktreeRuntimeLifecycleError {}

/// Hostable, synchronous Worktree runtime service.
///
/// The host decides when to call `poll`; this type never spawns a task. A poll
/// executes at most one cycle, making overlap impossible inside the service.
#[derive(Debug)]
pub struct WorktreeRuntimeService<C, W, X> {
    mode: WorktreeMode,
    policy: WorktreeRuntimePolicy,
    clock: C,
    watcher: W,
    executor: X,
    lifecycle: WorktreeRuntimeLifecycle,
    watcher_state: WorktreeRuntimeWatcherState,
    startup_cycle_pending: bool,
    cycle_in_progress: bool,
    pending_watcher_hints: usize,
    watcher_hints_observed: u64,
    debounce_deadline: Option<Duration>,
    next_periodic_cycle: Option<Duration>,
    cycles_completed: u64,
    cycles_failed: u64,
    last_cycle_cause: Option<WorktreeRuntimeCycleCause>,
    last_cycle: Option<WorktreeRuntimeLastCycle>,
}

impl<C, W, X> WorktreeRuntimeService<C, W, X>
where
    C: WorktreeRuntimeClock,
    W: WorktreeWatcher,
    X: WorktreeRuntimeCycle,
{
    /// Construct an inert service. Call `start` explicitly.
    #[must_use]
    pub fn new(
        mode: WorktreeMode,
        policy: WorktreeRuntimePolicy,
        clock: C,
        watcher: W,
        executor: X,
    ) -> Self {
        Self {
            mode,
            policy,
            clock,
            watcher,
            executor,
            lifecycle: WorktreeRuntimeLifecycle::Created,
            watcher_state: WorktreeRuntimeWatcherState::Disabled,
            startup_cycle_pending: false,
            cycle_in_progress: false,
            pending_watcher_hints: 0,
            watcher_hints_observed: 0,
            debounce_deadline: None,
            next_periodic_cycle: None,
            cycles_completed: 0,
            cycles_failed: 0,
            last_cycle_cause: None,
            last_cycle: None,
        }
    }

    /// Start lifecycle and optional watcher resources.
    pub fn start(&mut self) -> Result<WorktreeRuntimeStartSummary, WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => {}
            WorktreeRuntimeLifecycle::Shutdown => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown);
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyStarted);
            }
        }

        let now = self.clock.now();
        self.lifecycle = WorktreeRuntimeLifecycle::Running;
        self.startup_cycle_pending = self.mode.is_enabled();
        self.next_periodic_cycle = self
            .mode
            .is_enabled()
            .then(|| add_saturating(now, self.policy.periodic_cycle_interval));
        self.watcher_state = if self.mode.observes_local() {
            match self.watcher.start() {
                Ok(()) => WorktreeRuntimeWatcherState::Running,
                Err(_) => WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Start),
            }
        } else {
            WorktreeRuntimeWatcherState::Disabled
        };

        Ok(WorktreeRuntimeStartSummary {
            watcher: self.watcher_state,
            startup_cycle_pending: self.startup_cycle_pending,
        })
    }

    /// Request cooperative cancellation. No later poll may start a cycle.
    pub fn request_cancel(&mut self) -> Result<(), WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => Err(WorktreeRuntimeLifecycleError::NotStarted),
            WorktreeRuntimeLifecycle::Shutdown => {
                Err(WorktreeRuntimeLifecycleError::AlreadyShutdown)
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {
                self.lifecycle = WorktreeRuntimeLifecycle::Cancelling;
                Ok(())
            }
        }
    }

    /// Poll watcher hints and run at most one authoritative cycle.
    pub fn poll(&mut self) -> Result<WorktreeRuntimePoll, WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => {
                return Err(WorktreeRuntimeLifecycleError::NotStarted);
            }
            WorktreeRuntimeLifecycle::Shutdown => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown);
            }
            WorktreeRuntimeLifecycle::Cancelling => return Ok(WorktreeRuntimePoll::Cancelled),
            WorktreeRuntimeLifecycle::Running => {}
        }

        if !self.mode.is_enabled() {
            return Ok(WorktreeRuntimePoll::Disabled);
        }

        let now = self.clock.now();
        self.collect_watcher_hints(now);
        let Some(cause) = self.due_cycle(now) else {
            return Ok(WorktreeRuntimePoll::Idle);
        };
        Ok(self.run_one_cycle(cause, now))
    }

    /// Stop watcher resources and permanently close the service.
    pub fn shutdown(
        &mut self,
    ) -> Result<WorktreeRuntimeShutdownSummary, WorktreeRuntimeLifecycleError> {
        if self.lifecycle == WorktreeRuntimeLifecycle::Shutdown {
            return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown);
        }

        let watcher = match self.watcher_state {
            WorktreeRuntimeWatcherState::Running
            | WorktreeRuntimeWatcherState::Closed
            | WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Poll) => {
                match self.watcher.shutdown() {
                    Ok(()) => WorktreeRuntimeWatcherState::Stopped,
                    Err(_) => WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Shutdown),
                }
            }
            WorktreeRuntimeWatcherState::Disabled
            | WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Start)
            | WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Shutdown)
            | WorktreeRuntimeWatcherState::Stopped => WorktreeRuntimeWatcherState::Stopped,
        };
        self.watcher_state = watcher;
        self.lifecycle = WorktreeRuntimeLifecycle::Shutdown;
        self.startup_cycle_pending = false;
        self.pending_watcher_hints = 0;
        self.debounce_deadline = None;
        self.next_periodic_cycle = None;

        Ok(WorktreeRuntimeShutdownSummary {
            watcher,
            cycles_completed: self.cycles_completed,
        })
    }

    /// Safe count-only status snapshot.
    #[must_use]
    pub const fn status(&self) -> WorktreeRuntimeStatus {
        WorktreeRuntimeStatus {
            lifecycle: self.lifecycle,
            mode: self.mode,
            watcher: self.watcher_state,
            startup_cycle_pending: self.startup_cycle_pending,
            cycle_in_progress: self.cycle_in_progress,
            pending_watcher_hints: self.pending_watcher_hints,
            watcher_hints_observed: self.watcher_hints_observed,
            cycles_completed: self.cycles_completed,
            cycles_failed: self.cycles_failed,
            last_cycle_cause: self.last_cycle_cause,
            last_cycle: self.last_cycle,
        }
    }

    /// Borrow the host-provided executor for integration tests or composition.
    #[must_use]
    pub const fn executor(&self) -> &X {
        &self.executor
    }

    /// Mutably borrow the host-provided executor for explicit host/test setup.
    #[must_use]
    pub fn executor_mut(&mut self) -> &mut X {
        &mut self.executor
    }

    /// Mutably borrow the host-provided watcher for explicit test/composition control.
    #[must_use]
    pub fn watcher_mut(&mut self) -> &mut W {
        &mut self.watcher
    }

    fn collect_watcher_hints(&mut self, now: Duration) {
        if self.watcher_state != WorktreeRuntimeWatcherState::Running {
            return;
        }

        for _ in 0..self.policy.max_watcher_hints_per_poll {
            match self.watcher.poll_hint() {
                Ok(WorktreeWatcherPoll::Hint(_)) => {
                    self.pending_watcher_hints = self.pending_watcher_hints.saturating_add(1);
                    self.watcher_hints_observed = self.watcher_hints_observed.saturating_add(1);
                    self.debounce_deadline = Some(add_saturating(now, self.policy.debounce));
                }
                Ok(WorktreeWatcherPoll::Idle) => break,
                Ok(WorktreeWatcherPoll::Closed) => {
                    self.watcher_state = WorktreeRuntimeWatcherState::Closed;
                    break;
                }
                Err(_) => {
                    self.watcher_state =
                        WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Poll);
                    break;
                }
            }
        }
    }

    fn due_cycle(&self, now: Duration) -> Option<WorktreeRuntimeCycleCause> {
        if self.startup_cycle_pending {
            return Some(WorktreeRuntimeCycleCause::Startup);
        }
        if self
            .next_periodic_cycle
            .is_some_and(|deadline| now >= deadline)
        {
            return Some(WorktreeRuntimeCycleCause::Periodic);
        }
        if self.pending_watcher_hints > 0
            && self
                .debounce_deadline
                .is_some_and(|deadline| now >= deadline)
        {
            return Some(WorktreeRuntimeCycleCause::WatcherHint);
        }
        None
    }

    fn run_one_cycle(
        &mut self,
        cause: WorktreeRuntimeCycleCause,
        now: Duration,
    ) -> WorktreeRuntimePoll {
        debug_assert!(!self.cycle_in_progress);
        self.cycle_in_progress = true;
        self.last_cycle_cause = Some(cause);
        let request = WorktreeRuntimeCycleRequest {
            cause,
            mode: self.mode,
            full_scan_required: self.mode.observes_local(),
            import_enabled: self.mode.imports_local(),
            export_enabled: self.mode.exports_core(),
            budget: self.policy.budget,
            coalesced_watcher_hints: self.pending_watcher_hints,
        };
        let result = self.executor.run_cycle(request);
        self.cycle_in_progress = false;
        self.finish_attempt(now);

        match result {
            Ok(summary) => match validate_cycle_summary(request, summary) {
                Ok(()) => {
                    self.cycles_completed = self.cycles_completed.saturating_add(1);
                    self.last_cycle = Some(WorktreeRuntimeLastCycle::Completed(summary));
                    WorktreeRuntimePoll::CycleCompleted { cause, summary }
                }
                Err(violation) => {
                    self.cycles_failed = self.cycles_failed.saturating_add(1);
                    self.last_cycle = Some(WorktreeRuntimeLastCycle::Rejected(violation));
                    WorktreeRuntimePoll::CycleRejected { cause, violation }
                }
            },
            Err(failure) => {
                self.cycles_failed = self.cycles_failed.saturating_add(1);
                self.last_cycle = Some(WorktreeRuntimeLastCycle::Failed(failure));
                WorktreeRuntimePoll::CycleFailed { cause, failure }
            }
        }
    }

    fn finish_attempt(&mut self, now: Duration) {
        self.startup_cycle_pending = false;
        self.pending_watcher_hints = 0;
        self.debounce_deadline = None;
        self.next_periodic_cycle = Some(add_saturating(now, self.policy.periodic_cycle_interval));
    }
}

fn validate_cycle_summary(
    request: WorktreeRuntimeCycleRequest,
    summary: WorktreeRuntimeCycleSummary,
) -> Result<(), WorktreeRuntimeContractViolation> {
    if request.full_scan_required && !summary.full_scan_completed {
        return Err(WorktreeRuntimeContractViolation::RequiredScanMissing);
    }
    if !request.import_enabled
        && (summary.planned_imports > 0
            || summary.planned_deletes > 0
            || summary.submitted_imports > 0)
    {
        return Err(WorktreeRuntimeContractViolation::ImportModeViolation);
    }
    if !request.export_enabled && summary.applied_exports > 0 {
        return Err(WorktreeRuntimeContractViolation::ExportModeViolation);
    }
    if summary.planned_imports > request.budget.max_import_actions
        || summary.planned_deletes > request.budget.max_delete_candidates
        || summary.applied_exports > request.budget.max_export_actions
    {
        return Err(WorktreeRuntimeContractViolation::BudgetExceeded);
    }
    if summary.submitted_imports > summary.planned_imports {
        return Err(WorktreeRuntimeContractViolation::InvalidImportCounts);
    }
    Ok(())
}

fn add_saturating(base: Duration, delta: Duration) -> Duration {
    base.checked_add(delta).unwrap_or(Duration::MAX)
}
