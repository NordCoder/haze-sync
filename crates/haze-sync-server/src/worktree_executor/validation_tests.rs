use super::{validate_request, MAX_APPLICATION_CHANGE_LIMIT};
use haze_sync_worktree::{
    WorktreeMode, WorktreeRuntimeCycleBudget, WorktreeRuntimeCycleCause,
    WorktreeRuntimeCycleFailure, WorktreeRuntimeCycleRequest,
};

fn request(
    mode: WorktreeMode,
    full_scan_required: bool,
    import_enabled: bool,
    export_enabled: bool,
    imports: usize,
    deletes: usize,
    exports: usize,
) -> WorktreeRuntimeCycleRequest {
    WorktreeRuntimeCycleRequest {
        cause: WorktreeRuntimeCycleCause::Periodic,
        mode,
        full_scan_required,
        import_enabled,
        export_enabled,
        budget: WorktreeRuntimeCycleBudget {
            max_import_actions: imports,
            max_delete_candidates: deletes,
            max_export_actions: exports,
        },
        coalesced_watcher_hints: 0,
    }
}

#[test]
fn all_action_budgets_fail_closed_above_server_limit() {
    let too_many = MAX_APPLICATION_CHANGE_LIMIT + 1;
    for invalid in [
        request(WorktreeMode::ImportOnly, true, true, false, too_many, 1, 1),
        request(WorktreeMode::ImportOnly, true, true, false, 1, too_many, 1),
        request(WorktreeMode::ExportOnly, false, false, true, 1, 1, too_many),
    ] {
        assert_eq!(
            validate_request(invalid),
            Err(WorktreeRuntimeCycleFailure::Plan)
        );
    }
}

#[test]
fn dry_run_requires_full_scan_and_no_mutation_permissions() {
    for invalid in [
        request(WorktreeMode::DryRun, false, false, false, 1, 1, 1),
        request(WorktreeMode::DryRun, true, true, false, 1, 1, 1),
        request(WorktreeMode::DryRun, true, false, true, 1, 1, 1),
    ] {
        assert_eq!(
            validate_request(invalid),
            Err(WorktreeRuntimeCycleFailure::Plan)
        );
    }
}

#[test]
fn maximum_bounded_requests_remain_valid() {
    assert_eq!(
        validate_request(request(
            WorktreeMode::Bidirectional,
            true,
            true,
            true,
            MAX_APPLICATION_CHANGE_LIMIT,
            MAX_APPLICATION_CHANGE_LIMIT,
            MAX_APPLICATION_CHANGE_LIMIT,
        )),
        Ok(())
    );
    assert_eq!(
        validate_request(request(
            WorktreeMode::DryRun,
            true,
            false,
            false,
            MAX_APPLICATION_CHANGE_LIMIT,
            MAX_APPLICATION_CHANGE_LIMIT,
            MAX_APPLICATION_CHANGE_LIMIT,
        )),
        Ok(())
    );
}
