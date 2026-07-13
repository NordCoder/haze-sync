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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeMode {
    Disabled,
    ReadOnly,
    ImportOnly,
    ExportOnly,
    Bidirectional,
    DryRun,
}

impl WorktreeMode {
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    #[must_use]
    pub const fn runs_automatically(self) -> bool {
        !matches!(self, Self::Disabled | Self::DryRun)
    }

    #[must_use]
    pub const fn observes_local(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    #[must_use]
    pub const fn imports_local(self) -> bool {
        matches!(self, Self::ImportOnly | Self::Bidirectional)
    }

    #[must_use]
    pub const fn exports_core(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ExportOnly | Self::Bidirectional)
    }

    #[must_use]
    pub const fn allows_manual_planning(self) -> bool {
        matches!(self, Self::DryRun)
    }

    #[must_use]
    pub const fn permits_mutation(self) -> bool {
        self.imports_local() || self.exports_core()
    }
}

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
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeCycleBudget {
    pub max_import_actions: usize,
    pub max_delete_candidates: usize,
    pub max_export_actions: usize,
}

impl WorktreeRuntimeCycleBudget {
    fn is_valid(self) -> bool {
        self.max_import_actions > 0 && self.max_delete_candidates > 0 && self.max_export_actions > 0
    }
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
    #[must_use]
    pub const fn manual_dry_run(budget: WorktreeRuntimeCycleBudget) -> Self {
        Self {
            cause: WorktreeRuntimeCycleCause::Manual,
            mode: WorktreeMode::DryRun,
            full_scan_required: true,
            import_enabled: false,
            export_enabled: false,
            budget,
            coalesced_watcher_hints: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorktreeRuntimeManualRequest {
    pub full_scan_required: bool,
    pub budget: WorktreeRuntimeCycleBudget,
}

impl WorktreeRuntimeManualRequest {
    #[must_use]
    pub const fn new(full_scan_required: bool, budget: WorktreeRuntimeCycleBudget) -> Self {
        Self {
            full_scan_required,
            budget,
        }
    }

    #[must_use]
    pub const fn dry_run(budget: WorktreeRuntimeCycleBudget) -> Self {
        Self {
            full_scan_required: true,
            budget,
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

pub type WorktreeRuntimeCycleFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure>>
            + Send
            + 'a,
    >,
>;

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
        if !budget.is_valid() {
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
pub enum WorktreeRuntimeManualOutcome {
    NotStarted,
    Busy,
    Cancelling,
    Shutdown,
    Disabled,
    Completed(WorktreeRuntimeCycleSummary),
    Failed(WorktreeRuntimeCycleFailure),
    Rejected(WorktreeRuntimeContractViolation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorktreeRuntimeContractViolation {
    RequiredScanMissing,
    ImportModeViolation,
    ExportModeViolation,
    BudgetExceeded,
    InvalidImportCounts,
    InvalidManualRequest,
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
            Self::InvalidManualRequest => "worktree_runtime_invalid_manual_request",
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

struct CycleAttemptGuard<'a> {
    in_progress: &'a mut bool,
    last_cycle_cause: &'a mut Option<WorktreeRuntimeCycleCause>,
    previous_cause: Option<WorktreeRuntimeCycleCause>,
    committed: bool,
}

impl<'a> CycleAttemptGuard<'a> {
    fn begin(
        in_progress: &'a mut bool,
        last_cycle_cause: &'a mut Option<WorktreeRuntimeCycleCause>,
        cause: WorktreeRuntimeCycleCause,
    ) -> Self {
        let previous_cause = *last_cycle_cause;
        *in_progress = true;
        *last_cycle_cause = Some(cause);
        Self {
            in_progress,
            last_cycle_cause,
            previous_cause,
            committed: false,
        }
    }

    fn commit(mut self) {
        *self.in_progress = false;
        self.committed = true;
    }
}

impl Drop for CycleAttemptGuard<'_> {
    fn drop(&mut self) {
        *self.in_progress = false;
        if !self.committed {
            *self.last_cycle_cause = self.previous_cause;
        }
    }
}

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
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown)
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyStarted)
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

    pub async fn poll(&mut self) -> Result<WorktreeRuntimePoll, WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => {
                return Err(WorktreeRuntimeLifecycleError::NotStarted)
            }
            WorktreeRuntimeLifecycle::Shutdown => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown)
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
        Ok(self
            .run_one_cycle(self.automatic_request(cause), now, true)
            .await)
    }

    pub async fn run_manual_cycle(
        &mut self,
        manual: WorktreeRuntimeManualRequest,
    ) -> WorktreeRuntimeManualOutcome {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => return WorktreeRuntimeManualOutcome::NotStarted,
            WorktreeRuntimeLifecycle::Shutdown => return WorktreeRuntimeManualOutcome::Shutdown,
            WorktreeRuntimeLifecycle::Cancelling => {
                return WorktreeRuntimeManualOutcome::Cancelling
            }
            WorktreeRuntimeLifecycle::Running => {}
        }
        if self.cycle_in_progress {
            return WorktreeRuntimeManualOutcome::Busy;
        }
        if self.cancellation.is_cancelled() {
            self.lifecycle = WorktreeRuntimeLifecycle::Cancelling;
            return WorktreeRuntimeManualOutcome::Cancelling;
        }
        if self.mode == WorktreeMode::Disabled {
            return WorktreeRuntimeManualOutcome::Disabled;
        }
        if !manual.budget.is_valid()
            || (self.mode == WorktreeMode::DryRun && !manual.full_scan_required)
        {
            return WorktreeRuntimeManualOutcome::Rejected(
                WorktreeRuntimeContractViolation::InvalidManualRequest,
            );
        }
        let request = WorktreeRuntimeCycleRequest {
            cause: WorktreeRuntimeCycleCause::Manual,
            mode: self.mode,
            full_scan_required: manual.full_scan_required
                || self.mode.observes_local()
                || self.mode == WorktreeMode::DryRun,
            import_enabled: self.mode.imports_local(),
            export_enabled: self.mode.exports_core(),
            budget: manual.budget,
            coalesced_watcher_hints: 0,
        };
        match self.run_one_cycle(request, self.clock.now(), false).await {
            WorktreeRuntimePoll::CycleCompleted { summary, .. } => {
                WorktreeRuntimeManualOutcome::Completed(summary)
            }
            WorktreeRuntimePoll::CycleFailed { failure, .. } => {
                WorktreeRuntimeManualOutcome::Failed(failure)
            }
            WorktreeRuntimePoll::CycleRejected { violation, .. } => {
                WorktreeRuntimeManualOutcome::Rejected(violation)
            }
            _ => unreachable!("manual cycle always produces a cycle outcome"),
        }
    }

    pub fn shutdown(
        &mut self,
    ) -> Result<WorktreeRuntimeShutdownSummary, WorktreeRuntimeLifecycleError> {
        match self.lifecycle {
            WorktreeRuntimeLifecycle::Created => {
                return Err(WorktreeRuntimeLifecycleError::NotStarted)
            }
            WorktreeRuntimeLifecycle::Shutdown => {
                return Err(WorktreeRuntimeLifecycleError::AlreadyShutdown)
            }
            WorktreeRuntimeLifecycle::Running | WorktreeRuntimeLifecycle::Cancelling => {}
        }
        self.cancellation.cancel();
        self.watcher_state = match self.watcher_state {
            WorktreeRuntimeWatcherState::Running
            | WorktreeRuntimeWatcherState::Closed
            | WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Poll) => {
                match self.watcher.shutdown() {
                    Ok(()) => WorktreeRuntimeWatcherState::Stopped,
                    Err(_) => WorktreeRuntimeWatcherState::Failed(WorktreeWatcherFailure::Shutdown),
                }
            }
            _ => WorktreeRuntimeWatcherState::Stopped,
        };
        self.lifecycle = WorktreeRuntimeLifecycle::Shutdown;
        self.startup_cycle_pending = false;
        self.pending_watcher_hints = 0;
        self.debounce_deadline = None;
        self.next_periodic_cycle = None;
        Ok(WorktreeRuntimeShutdownSummary {
            watcher: self.watcher_state,
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

    fn automatic_request(&self, cause: WorktreeRuntimeCycleCause) -> WorktreeRuntimeCycleRequest {
        WorktreeRuntimeCycleRequest {
            cause,
            mode: self.mode,
            full_scan_required: self.mode.observes_local(),
            import_enabled: self.mode.imports_local(),
            export_enabled: self.mode.exports_core(),
            budget: self.policy.budget,
            coalesced_watcher_hints: self.pending_watcher_hints,
        }
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
        request: WorktreeRuntimeCycleRequest,
        now: Duration,
        advance_schedule: bool,
    ) -> WorktreeRuntimePoll {
        if self.cycle_in_progress {
            return WorktreeRuntimePoll::CycleRejected {
                cause: request.cause,
                violation: WorktreeRuntimeContractViolation::InvalidManualRequest,
            };
        }
        let guard = CycleAttemptGuard::begin(
            &mut self.cycle_in_progress,
            &mut self.last_cycle_cause,
            request.cause,
        );
        let result = self
            .executor
            .run_cycle(request, self.cancellation.clone())
            .await;
        guard.commit();
        if advance_schedule {
            self.finish_automatic_attempt(now);
        }
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
                cause: request.cause,
                failure: WorktreeRuntimeCycleFailure::Cancelled,
            };
        }
        match result {
            Ok(summary) => match validate_cycle_summary(request, summary) {
                Ok(()) => {
                    self.cycles_completed = self.cycles_completed.saturating_add(1);
                    self.last_cycle = Some(WorktreeRuntimeLastCycle::Completed(summary));
                    WorktreeRuntimePoll::CycleCompleted {
                        cause: request.cause,
                        summary,
                    }
                }
                Err(violation) => {
                    self.cycles_failed = self.cycles_failed.saturating_add(1);
                    self.last_cycle = Some(WorktreeRuntimeLastCycle::Rejected(violation));
                    WorktreeRuntimePoll::CycleRejected {
                        cause: request.cause,
                        violation,
                    }
                }
            },
            Err(failure) => {
                self.cycles_failed = self.cycles_failed.saturating_add(1);
                self.last_cycle = Some(WorktreeRuntimeLastCycle::Failed(failure));
                WorktreeRuntimePoll::CycleFailed {
                    cause: request.cause,
                    failure,
                }
            }
        }
    }

    fn finish_automatic_attempt(&mut self, now: Duration) {
        self.startup_cycle_pending = false;
        self.pending_watcher_hints = 0;
        self.debounce_deadline = None;
        self.next_periodic_cycle = if self.cancellation.is_cancelled() {
            None
        } else {
            Some(add_saturating(now, self.policy.periodic_cycle_interval))
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

fn add_saturating(left: Duration, right: Duration) -> Duration {
    left.checked_add(right).unwrap_or(Duration::MAX)
}
