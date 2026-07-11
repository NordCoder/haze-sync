//! Hostable Worktree runtime scheduling.
//!
//! The runtime is deliberately host-driven: it creates no unmanaged background
//! task and owns no Server startup wiring. Watcher events are path-free latency
//! hints only. Startup and periodic cycles still require authoritative full scans
//! whenever the selected mode observes local filesystem state.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Worktree adapter operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeMode {
    /// Runtime is completely inert.
    Disabled,
    /// Adapter may read Core/apply exports but must not write local facts to Core.
    ReadOnly,
    /// Observe and import local facts, but do not apply Core exports.
    ImportOnly,
    /// Apply Core exports, but do not watch or import local filesystem changes.
    ExportOnly,
    /// Observe/import local facts and apply Core exports.
    Bidirectional,
    /// Explicit planning-only representation for a future manual Server command.
    ///
    /// Dry-run never starts automatic cycles and grants no mutation,
    /// materialization, cursor-advance, trash, echo, or durable-state permission.
    DryRun,
}

impl WorktreeMode {
    /// Whether the mode is distinct from the completely disabled adapter.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Whether startup/periodic/watcher cycles may run automatically.
    #[must_use]
    pub const fn runs_automatically(self) -> bool {
        !matches!(self, Self::Disabled | Self::DryRun)
    }

    /// Whether this mode observes local state through authoritative scans.
    #[must_use]
    pub const fn observes_local(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    /// Whether local facts may be submitted to Core.
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

    /// Whether a future host may use this representation for manual planning.
    #[must_use]
    pub const fn allows_manual_planning(self) -> bool {
        matches!(self, Self::DryRun)
    }

    /// Whether any mutation permission is represented by this mode.
    #[must_use]
    pub const fn permits_mutation(self) -> bool {
        self.imports_local() || self.exports_core()
    }
}

/// Monotonic clock used by the host-driven scheduler.
pub trait WorktreeRuntimeClock {
    fn now(&self) -> Duration;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeWatcherHint {
    pub sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeWatcherPoll {
    Hint(WorktreeWatcherHint),
    Idle,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeWatcherFailure {
    Start,
    Poll,
    Shutdown,
}

impl WorktreeWatcherFailure {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Start => "worktree_watcher_start_failed",
            Self::Poll => "worktree_watcher_poll_failed",
            Self::Shutdown => "worktree_watcher_shutdown_failed",
        }
    }
}

pub trait WorktreeWatcher {
    fn start(&mut self) -> Result<(), WorktreeWatcherFailure>;
    fn poll_hint(&mut self) -> Result<WorktreeWatcherPoll, WorktreeWatcherFailure>;
    fn shutdown(&mut self) -> Result<(), WorktreeWatcherFailure>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeCycleCause {
    Startup,
    WatcherHint,
    Periodic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleBudget {
    pub max_import_actions: usize,
    pub max_delete_candidates: usize,
    pub max_export_actions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleRequest {
    pub cause: WorktreeRuntimeCycleCause,
    pub mode: WorktreeMode,
    pub full_scan_required: bool,
    pub import_enabled: bool,
    pub export_enabled: bool,
    pub budget: WorktreeRuntimeCycleBudget,
    pub coalesced_watcher_hints: usize,
}

impl WorktreeRuntimeCycleRequest {
    /// Build an explicit future manual dry-run request with no mutation rights.
    #[must_use]
    pub const fn manual_dry_run(
        budget: WorktreeRuntimeCycleBudget,
    ) -> WorktreeRuntimeCycleRequest {
        Self {
            cause: WorktreeRuntimeCycleCause::Periodic,
            mode: WorktreeMode::DryRun,
            full_scan_required: true,
            import_enabled: false,
            export_enabled: false,
            budget,
            coalesced_watcher_hints: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleSummary {
    pub full_scan_completed: bool,
    pub scanned_files: usize,
    pub skipped_entries: usize,
    pub planned_imports: usize,
    pub planned_deletes: usize,
    pub submitted_imports: usize,
    pub applied_exports: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeCycleFailure {
    Scan,
    Plan,
    Submit,
    Export,
    Cancelled,
}

impl WorktreeRuntimeCycleFailure {
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

/// Cooperative cancellation shared with a future Server-owned executor.
///
/// The token carries only one atomic boolean. It contains no paths, payloads,
/// cursors, credentials, or runtime handles.
#[derive(Clone, Default)]
pub struct WorktreeCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl WorktreeCancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl fmt::Debug for WorktreeCancellationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorktreeCancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Awaitable, `Send` cycle result without an async-trait dependency.
pub type WorktreeRuntimeCycleFuture<'a> = Pin<
    Box<
        dyn Future<
                Output = Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>,
            > + Send
            + 'a,
    >,
>;

/// Host-provided authoritative async cycle executor.
pub trait WorktreeRuntimeCycle {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimePolicy {
    debounce: Duration,
    periodic_cycle_interval: Duration,
    max_watcher_hints_per_poll: usize,
    budget: WorktreeRuntimeCycleBudget,
}

impl WorktreeRuntimePolicy {
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

    #[must_use]
    pub const fn debounce(self) -> Duration {
        self.debounce
    }

    #[must_use]
    pub const fn periodic_cycle_interval(self) -> Duration {
        self.periodic_cycle_interval
    }

    #[must_use]
    pub const fn max_watcher_hints_per_poll(self) -> usize {
        self.max_watcher_hints_per_poll
    }

    #[must_use]
    pub const fn budget(self) -> WorktreeRuntimeCycleBudget {
        self.budget
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimePolicyError {
    ZeroPeriodicInterval,
    ZeroHintBudget,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLifecycle {
    Created,
    Running,
    Cancelling,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeWatcherState {
    Disabled,
    Running,
    Closed,
    Failed(WorktreeWatcherFailure),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLastCycle {
    Completed(WorktreeRuntimeCycleSummary),
    Failed(WorktreeRuntimeCycleFailure),
    Rejected(WorktreeRuntimeContractViolation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeStatus {
    pub lifecycle: WorktreeRuntimeLifecycle,
    pub mode: WorktreeMode,
    pub watcher: WorktreeRuntimeWatcherState,
    pub startup_cycle_pending: bool,
    pub cycle_in_progress: bool,
    pub pending_watcher_hints: usize,
    pub watcher_hints_observed: u64,
    pub cycles_completed: u64,
    pub cycles_failed: u64,
    pub last_cycle_cause: Option<WorktreeRuntimeCycleCause>,
    pub last_cycle: Option<WorktreeRuntimeLastCycle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeStartSummary {
    pub watcher: WorktreeRuntimeWatcherState,
    pub startup_cycle_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeShutdownSummary {
    pub watcher: WorktreeRuntimeWatcherState,
    pub cycles_completed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimePoll {
    Idle,
    Disabled,
    DryRun,
    Cancelled,
    CycleCompleted {
        cause: WorktreeRuntimeCycleCause,
        summary: WorktreeRuntimeCycleSummary,
    },
    CycleFailed {
        cause: WorktreeRuntimeCycleCause,
        failure: WorktreeRuntimeCycleFailure,
    },
    CycleRejected {
        cause: WorktreeRuntimeCycleCause,
        violation: WorktreeRuntimeContractViolation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeContractViolation {
    RequiredScanMissing,
    ImportModeViolation,
    ExportModeViolation,
    BudgetExceeded,
    InvalidImportCounts,
}

impl WorktreeRuntimeContractViolation {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeLifecycleError {
    AlreadyStarted,
    AlreadyShutdown,
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

/// Hostable Worktree runtime service. It never spawns or detaches a task.
pub struct WorktreeRuntimeService<C, W, X> {
    mode: WorktreeMode,
    policy: WorktreeRuntimePolicy,
    clock: C,
    watcher: W,
    executor: X,
    cancellation: WorktreeCancellationToken,
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

impl<C, W, X> fmt::Debug for WorktreeRuntimeService<C, W, X> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorktreeRuntimeService")
            .field("mode", &self.mode)
            .field("policy", &self.policy)
            .field("lifecycle", &self.lifecycle)
            .field("watcher_state", &self.watcher_state)
            .field("startup_cycle_pending", &self.startup_cycle_pending)
            .field("cycle_in_progress", &self.cycle_in_progress)
            .field("pending_watcher_hints", &self.pending_watcher_hints)
            .field("watcher_hints_observed", &self.watcher_hints_observed)
            .field("cycles_completed", &self.cycles_completed)
            .field("cycles_failed", &self.cycles_failed)
            .field("last_cycle_cause", &self.last_cycle_cause)
            .field("last_cycle", &self.last_cycle)
            .field("cancelled", &self.cancellation.is_cancelled())
            .finish()
    }
}

impl<C, W, X> WorktreeRuntimeService<C, W, X>
where
    C: WorktreeRuntimeClock,
    W: WorktreeWatcher,
    X: WorktreeRuntimeCycle,
{
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
            cancellation: WorktreeCancellationToken::new(),
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
        self.startup_cycle_pending = self.mode.runs_automatically();
        self.next_periodic_cycle = self
            .mode
            .runs_automatically()
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

    /// Clone a path-free cancellation token for an async host or executor.
    #[must_use]
    pub fn cancellation_token(&self) -> WorktreeCancellationToken {
        self.cancellation.clone()
    }

    pub fn request_cancel(&mut self) -> Result<(), WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => Err(WorktreeRuntimeLifecycleError::NotStarted),
            WorktreeRuntimeLifecycle::Shutdown => {
                Err(WorktreeRuntimeLifecycleError::AlreadyShutdown)
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {
                self.cancellation.cancel();
                self.lifecycle = WorktreeRuntimeLifecycle::Cancelling;
                Ok(())
            }
        }
    }

    /// Poll watcher hints and await at most one authoritative cycle.
    pub async fn poll(
        &mut self,
    ) -> Result<WorktreeRuntimePoll, WorktreeRuntimeLifecycleError> {
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

        if self.cancellation.is_cancelled() {
            self.lifecycle = WorktreeRuntimeLifecycle::Cancelling;
            return Ok(WorktreeRuntimePoll::Cancelled);
        }
        match self.mode {
            WorktreeMode::Disabled => return Ok(WorktreeRuntimePoll::Disabled),
            WorktreeMode::DryRun => return Ok(WorktreeRuntimePoll::DryRun),
            _ => {}
        }

        let now = self.clock.now();
        self.collect_watcher_hints(now);
        let Some(cause) = self.due_cycle(now) else {
            return Ok(WorktreeRuntimePoll::Idle);
        };
        Ok(self.run_one_cycle(cause, now).await)
    }

    pub fn shutdown(
        &mut self,
    ) -> Result<WorktreeRuntimeShutdownSummary, WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => {
                return Err(WorktreeRuntimeLifecycleError::NotStarted);
            }
            WorktreeRuntimeLifecycle::Shutdown => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown);
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {}
        }

        self.cancellation.cancel();
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

    #[must_use]
    pub const fn executor(&self) -> &X {
        &self.executor
    }

    #[must_use]
    pub fn executor_mut(&mut self) -> &mut X {
        &mut self.executor
    }

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

    async fn run_one_cycle(
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
        let result = self
            .executor
            .run_cycle(request, self.cancellation.clone())
            .await;
        self.cycle_in_progress = false;
        self.finish_attempt(now);

        if self.cancellation.is_cancelled()
            || matches!(result, Err(WorktreeRuntimeCycleFailure::Cancelled))
        {
            self.cancellation.cancel();
            self.lifecycle = WorktreeRuntimeLifecycle::Cancelling;
            self.cycles_failed = self.cycles_failed.saturating_add(1);
            self.last_cycle = Some(WorktreeRuntimeLastCycle::Failed(
                WorktreeRuntimeCycleFailure::Cancelled,
            ));
            return WorktreeRuntimePoll::CycleFailed {
                cause,
                failure: WorktreeRuntimeCycleFailure::Cancelled,
            };
        }

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
        self.next_periodic_cycle = if self.cancellation.is_cancelled() {
            None
        } else {
            Some(add_saturating(
                now,
                self.policy.periodic_cycle_interval,
            ))
        };
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
