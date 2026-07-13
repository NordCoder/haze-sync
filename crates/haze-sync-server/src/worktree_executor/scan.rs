use super::{ServerWorktreeExecutorPolicy, MAX_APPLICATION_CHANGE_LIMIT};
use haze_sync_common::VaultPath;
use haze_sync_core::revision_service::compute_content_hash;
use haze_sync_worktree::{
    WorktreeConfig, WorktreeDeleteScan, WorktreeEchoGuard, WorktreeEchoGuardPolicy,
    WorktreeEchoStatus, WorktreeImportAction, WorktreeImportFile, WorktreeImportPlanner,
    WorktreeReconciler, WorktreeReconciliationKind, WorktreeReconciliationState,
    WorktreeReconciliationStateTransition, WorktreeRuntimeCycleFailure, WorktreeScanner,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    time::SystemTime,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct LocalObservation {
    pub(super) size: u64,
    pub(super) modified: Option<SystemTime>,
}

#[derive(Debug)]
pub(super) struct ScanStageOutput {
    pub(super) scanned_files: usize,
    pub(super) skipped_entries: usize,
    pub(super) import_actions: Vec<WorktreeImportAction>,
    pub(super) delete_scan: WorktreeDeleteScan,
    pub(super) observations: BTreeMap<VaultPath, LocalObservation>,
    pub(super) transitions: Vec<WorktreeReconciliationStateTransition>,
}

pub(super) fn run_scan_stage(
    config: WorktreeConfig,
    mut reconciliation: WorktreeReconciliationState,
    policy: ServerWorktreeExecutorPolicy,
    max_import_actions: usize,
    dry_run: bool,
) -> Result<ScanStageOutput, WorktreeRuntimeCycleFailure> {
    if max_import_actions == 0 || max_import_actions > MAX_APPLICATION_CHANGE_LIMIT {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }

    let scan = WorktreeScanner::new(config.clone())
        .scan()
        .map_err(|_| WorktreeRuntimeCycleFailure::Scan)?;
    let echo_policy = WorktreeEchoGuardPolicy::new(policy.echo_ttl, policy.echo_capacity)
        .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
    let mut echo_guard = WorktreeEchoGuard::new(echo_policy);
    if !dry_run {
        echo_guard
            .load_runtime_markers(&config, SystemTime::now())
            .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
    }
    let report = WorktreeReconciler::reconcile(
        &mut reconciliation,
        &scan,
        &mut echo_guard,
        SystemTime::now(),
    )
    .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;

    let eligible_paths: BTreeSet<VaultPath> = report
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                WorktreeReconciliationKind::Dirty | WorktreeReconciliationKind::Extra
            ) && entry.echo_status != WorktreeEchoStatus::Suppressed
        })
        .filter_map(|entry| entry.vault_path.clone())
        .collect();

    if scan.files.len().saturating_add(scan.skipped.len()) > policy.max_state_paths {
        return Err(WorktreeRuntimeCycleFailure::Plan);
    }

    let mut observations = BTreeMap::new();
    let mut import_files = Vec::new();
    for snapshot in &scan.files {
        observations.insert(
            snapshot.vault_path.clone(),
            LocalObservation {
                size: snapshot.size,
                modified: snapshot.modified,
            },
        );
        if !eligible_paths.contains(&snapshot.vault_path)
            || import_files.len() >= max_import_actions
        {
            continue;
        }
        let local_path = config
            .vault_path_to_local(&snapshot.vault_path)
            .map_err(|_| WorktreeRuntimeCycleFailure::Scan)?;
        let bytes = fs::read(local_path).map_err(|_| WorktreeRuntimeCycleFailure::Scan)?;
        if compute_content_hash(&bytes) != snapshot.content_hash {
            return Err(WorktreeRuntimeCycleFailure::Scan);
        }
        import_files.push(
            WorktreeImportFile::new(snapshot.clone(), bytes)
                .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?,
        );
    }

    let import_plan = WorktreeImportPlanner::plan(reconciliation.applied_state(), import_files)
        .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;
    let delete_scan = WorktreeDeleteScan::from_scan(reconciliation.applied_state(), &scan)
        .map_err(|_| WorktreeRuntimeCycleFailure::Plan)?;

    Ok(ScanStageOutput {
        scanned_files: scan.files.len(),
        skipped_entries: scan.skipped.len(),
        import_actions: import_plan.actions().to_vec(),
        delete_scan,
        observations,
        transitions: report.transitions().to_vec(),
    })
}
