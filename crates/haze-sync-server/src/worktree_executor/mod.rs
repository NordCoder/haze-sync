//! Server-owned execution of exactly one bounded Worktree cycle.
//!
//! The executor connects accepted Worktree filesystem primitives to Server
//! application services and passive Storage repositories. It owns no scheduler,
//! background task, startup hook, public DTO, or provider behavior.

mod exports;
mod imports;
mod scan;
mod state;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;

use crate::application::ServerApplicationServices;
use haze_sync_common::{AdapterId, Sha256};
use haze_sync_worktree::{
    WorktreeCancellationToken, WorktreeConfig, WorktreeEchoGuardPolicy, WorktreeMode,
    WorktreeRuntimeCycle, WorktreeRuntimeCycleFailure, WorktreeRuntimeCycleFuture,
    WorktreeRuntimeCycleRequest, WorktreeRuntimeCycleSummary, WorktreeTrashPolicy,
};
use sqlx::PgPool;
use std::{fmt, path::PathBuf, time::Duration};

use scan::run_scan_stage;

const MAX_APPLICATION_CHANGE_LIMIT: usize = 1_000;
const MAX_APPLICATION_CHANGE_LIMIT_U32: u32 = 1_000;
const MAX_DELETE_RATIO_BASIS_POINTS: u16 = 10_000;

/// Bounded, validated policy used by one executor instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ServerWorktreeExecutorPolicy {
    state_page_size: u32,
    max_state_paths: usize,
    max_delete_ratio_basis_points: u16,
    echo_ttl: Duration,
    echo_capacity: usize,
    trash_retention: Duration,
}

impl ServerWorktreeExecutorPolicy {
    pub(crate) fn new(
        state_page_size: u32,
        max_state_paths: usize,
        max_delete_ratio_basis_points: u16,
        echo_ttl: Duration,
        echo_capacity: usize,
        trash_retention: Duration,
    ) -> Result<Self, ServerWorktreeExecutorConfigError> {
        if state_page_size == 0
            || state_page_size > MAX_APPLICATION_CHANGE_LIMIT_U32
            || max_state_paths < state_page_size as usize
            || max_delete_ratio_basis_points > MAX_DELETE_RATIO_BASIS_POINTS
            || echo_ttl.is_zero()
            || echo_capacity == 0
            || trash_retention.is_zero()
            || trash_retention.subsec_nanos() != 0
        {
            return Err(ServerWorktreeExecutorConfigError::InvalidPolicy);
        }
        WorktreeEchoGuardPolicy::new(echo_ttl, echo_capacity)
            .map_err(|_| ServerWorktreeExecutorConfigError::InvalidPolicy)?;
        WorktreeTrashPolicy::new(trash_retention)
            .map_err(|_| ServerWorktreeExecutorConfigError::InvalidPolicy)?;

        Ok(Self {
            state_page_size,
            max_state_paths,
            max_delete_ratio_basis_points,
            echo_ttl,
            echo_capacity,
            trash_retention,
        })
    }
}

impl Default for ServerWorktreeExecutorPolicy {
    fn default() -> Self {
        Self {
            state_page_size: 250,
            max_state_paths: 10_000,
            max_delete_ratio_basis_points: 500,
            echo_ttl: Duration::from_secs(5 * 60),
            echo_capacity: 10_000,
            trash_retention: Duration::from_secs(30 * 24 * 60 * 60),
        }
    }
}

/// Safe construction failures. Formatting never includes the configured root or fingerprint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ServerWorktreeExecutorConfigError {
    InvalidWorktreeRoot,
    InvalidPolicy,
}

impl fmt::Display for ServerWorktreeExecutorConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidWorktreeRoot => "worktree executor root is invalid",
            Self::InvalidPolicy => "worktree executor policy is invalid",
        })
    }
}

impl std::error::Error for ServerWorktreeExecutorConfigError {}

/// Executes one explicitly requested, bounded Worktree cycle.
pub(crate) struct ServerWorktreeCycleExecutor {
    services: ServerApplicationServices,
    pool: PgPool,
    adapter_id: AdapterId,
    config: WorktreeConfig,
    root_fingerprint: Sha256,
    policy: ServerWorktreeExecutorPolicy,
    #[cfg(test)]
    failure_injection: FailureInjection,
}

impl ServerWorktreeCycleExecutor {
    pub(crate) fn new(
        services: ServerApplicationServices,
        pool: PgPool,
        adapter_id: AdapterId,
        root: PathBuf,
        root_fingerprint: Sha256,
        policy: ServerWorktreeExecutorPolicy,
    ) -> Result<Self, ServerWorktreeExecutorConfigError> {
        let config = WorktreeConfig::new(root)
            .map_err(|_| ServerWorktreeExecutorConfigError::InvalidWorktreeRoot)?;
        // Revalidate values when a policy was assembled through Default.
        let policy = ServerWorktreeExecutorPolicy::new(
            policy.state_page_size,
            policy.max_state_paths,
            policy.max_delete_ratio_basis_points,
            policy.echo_ttl,
            policy.echo_capacity,
            policy.trash_retention,
        )?;

        Ok(Self {
            services,
            pool,
            adapter_id,
            config,
            root_fingerprint,
            policy,
            #[cfg(test)]
            failure_injection: FailureInjection::default(),
        })
    }

    async fn execute_cycle(
        &mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> Result<WorktreeRuntimeCycleSummary, WorktreeRuntimeCycleFailure> {
        validate_request(request)?;
        ensure_not_cancelled(&cancellation)?;
        self.verify_required_binding().await?;
        ensure_not_cancelled(&cancellation)?;

        let loaded = self.load_durable_state(&cancellation).await?;
        let mut applied_state = loaded.applied;
        let mut summary = WorktreeRuntimeCycleSummary::default();

        let scan_stage = if request.full_scan_required {
            ensure_not_cancelled(&cancellation)?;
            let config = self.config.clone();
            let policy = self.policy;
            let reconciliation = loaded.reconciliation;
            let max_import_actions = request.budget.max_import_actions;
            let dry_run = request.mode == WorktreeMode::DryRun;
            let stage = tokio::task::spawn_blocking(move || {
                run_scan_stage(config, reconciliation, policy, max_import_actions, dry_run)
            })
            .await
            .map_err(|_| WorktreeRuntimeCycleFailure::Scan)??;
            ensure_not_cancelled(&cancellation)?;

            summary.full_scan_completed = true;
            summary.scanned_files = stage.scanned_files;
            summary.skipped_entries = stage.skipped_entries;
            Some(stage)
        } else {
            None
        };

        let (import_actions, delete_candidates) = if let Some(stage) = scan_stage.as_ref() {
            if request.import_enabled {
                self.persist_observation_transitions(&stage.transitions, &cancellation)
                    .await?;
            }

            let import_count = stage
                .import_actions
                .len()
                .min(request.budget.max_import_actions);
            summary.planned_imports = import_count;

            let delete_count = stage.delete_scan.candidates().len();
            if request.mode == WorktreeMode::DryRun {
                summary.planned_deletes = delete_count.min(request.budget.max_delete_candidates);
                (Vec::new(), Vec::new())
            } else if request.import_enabled {
                let deletes = self.authorize_delete_candidates(
                    stage.delete_scan.clone(),
                    request.budget.max_delete_candidates,
                )?;
                summary.planned_deletes = deletes.len();
                (stage.import_actions[..import_count].to_vec(), deletes)
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        if request.mode == WorktreeMode::DryRun {
            return Ok(summary);
        }

        if request.import_enabled {
            let observations = scan_stage
                .as_ref()
                .map(|stage| &stage.observations)
                .ok_or(WorktreeRuntimeCycleFailure::Plan)?;
            self.submit_imports(
                &import_actions,
                observations,
                &mut applied_state,
                &cancellation,
                &mut summary,
            )
            .await?;
            self.submit_deletes(&delete_candidates, &mut applied_state, &cancellation)
                .await?;
        }

        if request.export_enabled {
            self.apply_exports(
                request.budget.max_export_actions,
                &mut applied_state,
                &cancellation,
                &mut summary,
            )
            .await?;
        }

        Ok(summary)
    }
}

impl fmt::Debug for ServerWorktreeCycleExecutor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerWorktreeCycleExecutor")
            .field("adapter_id", &self.adapter_id)
            .field("root", &"[REDACTED]")
            .field("root_fingerprint", &"[REDACTED]")
            .field("database", &"[REDACTED]")
            .field("services", &"[REDACTED]")
            .field("policy", &self.policy)
            .finish()
    }
}

impl WorktreeRuntimeCycle for ServerWorktreeCycleExecutor {
    fn run_cycle<'a>(
        &'a mut self,
        request: WorktreeRuntimeCycleRequest,
        cancellation: WorktreeCancellationToken,
    ) -> WorktreeRuntimeCycleFuture<'a> {
        Box::pin(async move { self.execute_cycle(request, cancellation).await })
    }
}

fn validate_request(
    request: WorktreeRuntimeCycleRequest,
) -> Result<(), WorktreeRuntimeCycleFailure> {
    if request.budget.max_import_actions == 0
        || request.budget.max_delete_candidates == 0
        || request.budget.max_export_actions == 0
        || request.budget.max_import_actions > MAX_APPLICATION_CHANGE_LIMIT
        || request.budget.max_delete_candidates > MAX_APPLICATION_CHANGE_LIMIT
        || request.budget.max_export_actions > MAX_APPLICATION_CHANGE_LIMIT
    {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }
    if request.import_enabled && !request.mode.imports_local() {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }
    if request.export_enabled && !request.mode.exports_core() {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }
    if request.import_enabled && !request.full_scan_required {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }
    if request.mode == WorktreeMode::DryRun
        && (!request.full_scan_required || request.import_enabled || request.export_enabled)
    {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }
    Ok(())
}

fn ensure_not_cancelled(
    cancellation: &WorktreeCancellationToken,
) -> Result<(), WorktreeRuntimeCycleFailure> {
    if cancellation.is_cancelled() {
        Err(WorktreeRuntimeCycleFailure::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
struct FailureInjection {
    fail_after_import_submission_once: bool,
    fail_after_export_filesystem_once: bool,
    cancel_after_import_commit_once: bool,
}
