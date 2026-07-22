//! Long-running, mode-aware runtime scheduler for the Google Drive adapter.
//!
//! Provider/Core composition is injected through [`RuntimeCycleExecutor`]. The
//! scheduler owns lifecycle, full-scan cadence, retry backoff, stable cycle IDs,
//! checkpoint enforcement, cooperative shutdown, and restart metadata. Policy
//! remains in the existing scan/change/export/delete modules and Core/API.

use crate::config::{AdapterConfig, AdapterMode};
use crate::error::RuntimeError;
use crate::identity::AdapterIdentity;
use std::cmp;
use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(300);
const MAX_CYCLE_ID: u64 = u64::MAX - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Created,
    Running,
    ShutdownRequested,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupStatus {
    pub mode: AdapterMode,
    pub poll_interval_seconds: u64,
    pub full_scan_interval_seconds: u64,
    pub max_deletes_per_run: u32,
    pub max_delete_ratio_percent: u8,
}

impl StartupStatus {
    pub fn from_config(config: &AdapterConfig) -> Self {
        Self {
            mode: config.mode,
            poll_interval_seconds: config.intervals.poll_interval_seconds(),
            full_scan_interval_seconds: config.intervals.full_scan_interval_seconds(),
            max_deletes_per_run: config.delete_safety.max_deletes_per_run,
            max_delete_ratio_percent: config.delete_safety.max_delete_ratio_percent,
        }
    }

    pub const fn is_dry_run(&self) -> bool {
        self.mode.is_dry_run_mode()
    }
}

impl fmt::Display for StartupStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "mode={}, dry_run={}, poll_interval_seconds={}, full_scan_interval_seconds={}, max_deletes_per_run={}, max_delete_ratio_percent={}",
            self.mode,
            self.is_dry_run(),
            self.poll_interval_seconds,
            self.full_scan_interval_seconds,
            self.max_deletes_per_run,
            self.max_delete_ratio_percent
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCycleKind {
    FullScan,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCycleContext {
    pub cycle_id: u64,
    pub kind: RuntimeCycleKind,
    pub mode: AdapterMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeCycleStatistics {
    pub provider_reads: u64,
    pub core_reads: u64,
    pub core_writes: u64,
    pub provider_writes: u64,
    pub delete_candidates: u64,
    pub unsupported_entries: u64,
}

impl RuntimeCycleStatistics {
    fn accumulate(&mut self, other: Self) {
        self.provider_reads = self.provider_reads.saturating_add(other.provider_reads);
        self.core_reads = self.core_reads.saturating_add(other.core_reads);
        self.core_writes = self.core_writes.saturating_add(other.core_writes);
        self.provider_writes = self.provider_writes.saturating_add(other.provider_writes);
        self.delete_candidates = self
            .delete_candidates
            .saturating_add(other.delete_candidates);
        self.unsupported_entries = self
            .unsupported_entries
            .saturating_add(other.unsupported_entries);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCycleOutcome {
    pub statistics: RuntimeCycleStatistics,
    /// Must be true after every successful mutation-capable cycle. Read-only and
    /// dry-run modes cannot mutate durable state and therefore leave it false.
    pub checkpoint_committed: bool,
}

impl RuntimeCycleOutcome {
    #[must_use]
    pub const fn read_only(statistics: RuntimeCycleStatistics) -> Self {
        Self {
            statistics,
            checkpoint_committed: false,
        }
    }

    #[must_use]
    pub const fn committed(statistics: RuntimeCycleStatistics) -> Self {
        Self {
            statistics,
            checkpoint_committed: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCycleErrorCategory {
    Authentication,
    ProviderUnavailable,
    CoreUnavailable,
    DurableState,
    InvalidState,
    CompositionUnavailable,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCycleError {
    category: RuntimeCycleErrorCategory,
    retryable: bool,
    safe_message: &'static str,
}

impl RuntimeCycleError {
    #[must_use]
    pub const fn new(
        category: RuntimeCycleErrorCategory,
        retryable: bool,
        safe_message: &'static str,
    ) -> Self {
        Self {
            category,
            retryable,
            safe_message,
        }
    }

    #[must_use]
    pub const fn category(&self) -> RuntimeCycleErrorCategory {
        self.category
    }

    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for RuntimeCycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.safe_message)
    }
}

impl std::error::Error for RuntimeCycleError {}

/// One complete adapter cycle. Implementations must not expose private content
/// through errors or debug output. A retry of the same `cycle_id` must be safe.
pub trait RuntimeCycleExecutor {
    fn execute_cycle(
        &mut self,
        context: RuntimeCycleContext,
    ) -> Result<RuntimeCycleOutcome, RuntimeCycleError>;
}

pub trait RuntimeSleeper {
    fn sleep(&mut self, duration: Duration);
}

#[derive(Debug, Default)]
pub struct ThreadRuntimeSleeper;

impl RuntimeSleeper for ThreadRuntimeSleeper {
    fn sleep(&mut self, duration: Duration) {
        thread::sleep(duration);
    }
}

#[derive(Clone, Debug)]
pub struct ShutdownSignal {
    requested: Arc<AtomicBool>,
}

impl ShutdownSignal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
        }
    }

    #[must_use]
    pub fn handler_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.requested)
    }

    pub fn request(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeResumeState {
    pub next_cycle_id: u64,
    pub elapsed_since_full_scan: Duration,
}

impl Default for RuntimeResumeState {
    fn default() -> Self {
        Self {
            next_cycle_id: 1,
            elapsed_since_full_scan: Duration::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLoopOptions {
    pub max_iterations: Option<u64>,
    pub max_backoff: Duration,
    pub resume: RuntimeResumeState,
}

impl Default for RuntimeLoopOptions {
    fn default() -> Self {
        Self {
            max_iterations: None,
            max_backoff: DEFAULT_MAX_BACKOFF,
            resume: RuntimeResumeState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeLoopSummary {
    pub successful_cycles: u64,
    pub retryable_failures: u64,
    pub statistics: RuntimeCycleStatistics,
    pub resume: RuntimeResumeState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLoopError {
    Cycle(RuntimeCycleError),
    MissingDurableCheckpoint { cycle_id: u64 },
    CycleIdExhausted,
}

impl fmt::Display for RuntimeLoopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle(error) => write!(formatter, "runtime cycle failed safely: {error}"),
            Self::MissingDurableCheckpoint { .. } => formatter.write_str(
                "mutation-capable runtime cycle completed without a durable checkpoint",
            ),
            Self::CycleIdExhausted => {
                formatter.write_str("runtime cycle identifier space exhausted")
            }
        }
    }
}

impl std::error::Error for RuntimeLoopError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cycle(error) => Some(error),
            Self::MissingDurableCheckpoint { .. } | Self::CycleIdExhausted => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AdapterRuntime {
    config: AdapterConfig,
    identity: AdapterIdentity,
    state: RuntimeState,
}

impl fmt::Debug for AdapterRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdapterRuntime")
            .field("config", &"<redacted-adapter-config>")
            .field("identity", &"<redacted-adapter-identity>")
            .field("state", &self.state)
            .finish()
    }
}

impl AdapterRuntime {
    pub fn from_env() -> Result<Self, RuntimeError> {
        let config = AdapterConfig::load_from_env()?;
        let identity = AdapterIdentity::load_from_env()?;
        Ok(Self::new(config, identity))
    }

    #[must_use]
    pub const fn new(config: AdapterConfig, identity: AdapterIdentity) -> Self {
        Self {
            config,
            identity,
            state: RuntimeState::Created,
        }
    }

    #[must_use]
    pub const fn config(&self) -> &AdapterConfig {
        &self.config
    }

    #[must_use]
    pub const fn identity(&self) -> &AdapterIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn state(&self) -> RuntimeState {
        self.state
    }

    pub fn start(&mut self) -> Result<StartupStatus, RuntimeError> {
        self.state = RuntimeState::Running;
        Ok(StartupStatus::from_config(&self.config))
    }

    pub fn request_shutdown(&mut self) {
        if self.state == RuntimeState::Running {
            self.state = RuntimeState::ShutdownRequested;
        }
    }

    pub fn stop(&mut self) {
        self.state = RuntimeState::Stopped;
    }

    pub fn run_loop<E: RuntimeCycleExecutor, S: RuntimeSleeper>(
        &mut self,
        executor: &mut E,
        sleeper: &mut S,
        shutdown: &ShutdownSignal,
        options: RuntimeLoopOptions,
    ) -> Result<RuntimeLoopSummary, RuntimeLoopError> {
        if self.state != RuntimeState::Running {
            return Err(RuntimeLoopError::Cycle(RuntimeCycleError::new(
                RuntimeCycleErrorCategory::InvalidState,
                false,
                "runtime must be started before entering the loop",
            )));
        }

        let mut iterations = 0_u64;
        let mut successful_cycles = 0_u64;
        let mut retryable_failures = 0_u64;
        let mut statistics = RuntimeCycleStatistics::default();
        let mut resume = options.resume;
        let poll_interval = self.config.intervals.poll_interval;
        let full_scan_interval = self.config.intervals.full_scan_interval;
        let mut backoff = poll_interval;

        while !shutdown.is_requested()
            && options
                .max_iterations
                .is_none_or(|maximum| iterations < maximum)
        {
            iterations = iterations.saturating_add(1);

            if self.config.mode == AdapterMode::Disabled {
                sleeper.sleep(poll_interval);
                resume.elapsed_since_full_scan = resume
                    .elapsed_since_full_scan
                    .saturating_add(poll_interval);
                continue;
            }

            let kind = if resume.elapsed_since_full_scan >= full_scan_interval {
                RuntimeCycleKind::FullScan
            } else {
                RuntimeCycleKind::Incremental
            };
            let context = RuntimeCycleContext {
                cycle_id: resume.next_cycle_id,
                kind,
                mode: self.config.mode,
            };

            match executor.execute_cycle(context) {
                Ok(outcome) => {
                    if self.config.mode.permits_durable_state_mutation()
                        && !outcome.checkpoint_committed
                    {
                        return Err(RuntimeLoopError::MissingDurableCheckpoint {
                            cycle_id: context.cycle_id,
                        });
                    }
                    statistics.accumulate(outcome.statistics);
                    successful_cycles = successful_cycles.saturating_add(1);
                    if context.cycle_id >= MAX_CYCLE_ID {
                        return Err(RuntimeLoopError::CycleIdExhausted);
                    }
                    resume.next_cycle_id = context.cycle_id + 1;
                    if kind == RuntimeCycleKind::FullScan {
                        resume.elapsed_since_full_scan = Duration::ZERO;
                    }
                    backoff = poll_interval;
                    sleeper.sleep(poll_interval);
                    resume.elapsed_since_full_scan = resume
                        .elapsed_since_full_scan
                        .saturating_add(poll_interval);
                }
                Err(error) if error.is_retryable() => {
                    retryable_failures = retryable_failures.saturating_add(1);
                    sleeper.sleep(backoff);
                    resume.elapsed_since_full_scan =
                        resume.elapsed_since_full_scan.saturating_add(backoff);
                    backoff = cmp::min(backoff.saturating_mul(2), options.max_backoff);
                }
                Err(error) => return Err(RuntimeLoopError::Cycle(error)),
            }
        }

        if shutdown.is_requested() {
            self.request_shutdown();
        }

        Ok(RuntimeLoopSummary {
            successful_cycles,
            retryable_failures,
            statistics,
            resume,
        })
    }
}

/// Production guard used until the concrete provider/Core composition is
/// published. It prevents enabled modes from reporting false success.
#[derive(Debug, Default)]
pub struct GuardedProductionCycleExecutor;

impl RuntimeCycleExecutor for GuardedProductionCycleExecutor {
    fn execute_cycle(
        &mut self,
        _context: RuntimeCycleContext,
    ) -> Result<RuntimeCycleOutcome, RuntimeCycleError> {
        Err(RuntimeCycleError::new(
            RuntimeCycleErrorCategory::CompositionUnavailable,
            false,
            "Google Drive provider/Core cycle composition is not enabled",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeleteSafetyConfig, RuntimeIntervals, SecretPath, SecretString};
    use std::collections::VecDeque;

    fn config(mode: AdapterMode) -> AdapterConfig {
        AdapterConfig {
            server_url: "https://sentinel-runtime-endpoint.example.test".to_owned(),
            adapter_token: SecretString::from_raw("test", "sentinel-runtime-token").unwrap(),
            drive_root_folder_id: "sentinel-runtime-provider-root".to_owned(),
            oauth_token_path: SecretPath::from_raw(
                "test",
                "/run/secrets/sentinel-runtime-oauth.json",
            )
            .unwrap(),
            mode,
            intervals: RuntimeIntervals::new(Duration::from_secs(10), Duration::from_secs(30))
                .unwrap(),
            delete_safety: DeleteSafetyConfig::new(5, 20).unwrap(),
        }
    }

    fn identity() -> AdapterIdentity {
        AdapterIdentity::from_raw("sentinel-runtime-identity").unwrap()
    }

    #[derive(Default)]
    struct FakeSleeper {
        sleeps: Vec<Duration>,
    }

    impl RuntimeSleeper for FakeSleeper {
        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
        }
    }

    #[derive(Default)]
    struct FakeExecutor {
        outcomes: VecDeque<Result<RuntimeCycleOutcome, RuntimeCycleError>>,
        contexts: Vec<RuntimeCycleContext>,
    }

    impl RuntimeCycleExecutor for FakeExecutor {
        fn execute_cycle(
            &mut self,
            context: RuntimeCycleContext,
        ) -> Result<RuntimeCycleOutcome, RuntimeCycleError> {
            self.contexts.push(context);
            self.outcomes.pop_front().unwrap_or_else(|| {
                Ok(if context.mode.permits_durable_state_mutation() {
                    RuntimeCycleOutcome::committed(RuntimeCycleStatistics::default())
                } else {
                    RuntimeCycleOutcome::read_only(RuntimeCycleStatistics::default())
                })
            })
        }
    }

    #[test]
    fn runtime_start_returns_safe_status() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::ImportOnly), identity());
        let status = runtime.start().unwrap();

        assert_eq!(runtime.state(), RuntimeState::Running);
        assert_eq!(
            runtime.identity().expose_for_route(),
            "sentinel-runtime-identity"
        );
        assert_eq!(status.mode, AdapterMode::ImportOnly);
        assert!(!status.is_dry_run());
        assert_eq!(status.poll_interval_seconds, 10);
        assert_eq!(status.full_scan_interval_seconds, 30);
    }

    #[test]
    fn disabled_mode_stays_alive_without_executing_provider_or_core_work() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::Disabled), identity());
        runtime.start().unwrap();
        let mut executor = FakeExecutor::default();
        let mut sleeper = FakeSleeper::default();

        let summary = runtime
            .run_loop(
                &mut executor,
                &mut sleeper,
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(3),
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap();

        assert!(executor.contexts.is_empty());
        assert_eq!(summary.successful_cycles, 0);
        assert_eq!(sleeper.sleeps, vec![Duration::from_secs(10); 3]);
    }

    #[test]
    fn first_cycle_is_full_scan_then_incremental_until_scan_is_due_again() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::DryRun), identity());
        runtime.start().unwrap();
        let mut executor = FakeExecutor::default();
        let mut sleeper = FakeSleeper::default();

        let summary = runtime
            .run_loop(
                &mut executor,
                &mut sleeper,
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(4),
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap();

        let kinds = executor
            .contexts
            .iter()
            .map(|context| context.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                RuntimeCycleKind::FullScan,
                RuntimeCycleKind::Incremental,
                RuntimeCycleKind::Incremental,
                RuntimeCycleKind::FullScan,
            ]
        );
        assert_eq!(summary.successful_cycles, 4);
        assert_eq!(summary.resume.next_cycle_id, 5);
    }

    #[test]
    fn retryable_failure_reuses_cycle_id_and_applies_bounded_backoff() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::ImportOnly), identity());
        runtime.start().unwrap();
        let retryable = RuntimeCycleError::new(
            RuntimeCycleErrorCategory::ProviderUnavailable,
            true,
            "provider temporarily unavailable",
        );
        let statistics = RuntimeCycleStatistics {
            provider_reads: 1,
            core_writes: 1,
            ..RuntimeCycleStatistics::default()
        };
        let mut executor = FakeExecutor {
            outcomes: VecDeque::from([
                Err(retryable.clone()),
                Err(retryable),
                Ok(RuntimeCycleOutcome::committed(statistics)),
            ]),
            contexts: Vec::new(),
        };
        let mut sleeper = FakeSleeper::default();

        let summary = runtime
            .run_loop(
                &mut executor,
                &mut sleeper,
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(3),
                    max_backoff: Duration::from_secs(15),
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap();

        assert_eq!(summary.retryable_failures, 2);
        assert_eq!(summary.successful_cycles, 1);
        assert_eq!(summary.statistics, statistics);
        assert_eq!(
            executor
                .contexts
                .iter()
                .map(|context| context.cycle_id)
                .collect::<Vec<_>>(),
            vec![1, 1, 1]
        );
        assert_eq!(
            sleeper.sleeps,
            vec![
                Duration::from_secs(10),
                Duration::from_secs(15),
                Duration::from_secs(10),
            ]
        );
    }

    #[test]
    fn mutation_capable_mode_cannot_advance_without_durable_checkpoint() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::Bidirectional), identity());
        runtime.start().unwrap();
        let mut executor = FakeExecutor {
            outcomes: VecDeque::from([Ok(RuntimeCycleOutcome::read_only(
                RuntimeCycleStatistics::default(),
            ))]),
            contexts: Vec::new(),
        };
        let error = runtime
            .run_loop(
                &mut executor,
                &mut FakeSleeper::default(),
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(1),
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap_err();

        assert_eq!(
            error,
            RuntimeLoopError::MissingDurableCheckpoint { cycle_id: 1 }
        );
    }

    #[test]
    fn resume_state_preserves_monotonic_cycle_identity_across_restart() {
        let mut first = AdapterRuntime::new(config(AdapterMode::DryRun), identity());
        first.start().unwrap();
        let first_summary = first
            .run_loop(
                &mut FakeExecutor::default(),
                &mut FakeSleeper::default(),
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(1),
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap();

        let mut restarted = AdapterRuntime::new(config(AdapterMode::DryRun), identity());
        restarted.start().unwrap();
        let mut executor = FakeExecutor::default();
        let second_summary = restarted
            .run_loop(
                &mut executor,
                &mut FakeSleeper::default(),
                &ShutdownSignal::new(),
                RuntimeLoopOptions {
                    max_iterations: Some(1),
                    resume: first_summary.resume,
                    ..RuntimeLoopOptions::default()
                },
            )
            .unwrap();

        assert_eq!(executor.contexts[0].cycle_id, 2);
        assert_eq!(second_summary.resume.next_cycle_id, 3);
    }

    #[test]
    fn shutdown_is_cooperative_and_status_surfaces_remain_redacted() {
        let mut runtime = AdapterRuntime::new(config(AdapterMode::Disabled), identity());
        let status = runtime.start().unwrap();
        let shutdown = ShutdownSignal::new();
        shutdown.request();
        let summary = runtime
            .run_loop(
                &mut FakeExecutor::default(),
                &mut FakeSleeper::default(),
                &shutdown,
                RuntimeLoopOptions::default(),
            )
            .unwrap();

        assert_eq!(runtime.state(), RuntimeState::ShutdownRequested);
        assert_eq!(summary.successful_cycles, 0);
        runtime.stop();
        assert_eq!(runtime.state(), RuntimeState::Stopped);

        let rendered = format!("{runtime:?} {status}");
        for sentinel in [
            "sentinel-runtime-endpoint.example.test",
            "sentinel-runtime-token",
            "sentinel-runtime-provider-root",
            "/run/secrets/sentinel-runtime-oauth.json",
            "sentinel-runtime-identity",
        ] {
            assert!(!rendered.contains(sentinel));
        }
    }
}
