REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: server-api-p8-sync-once-race-fix-20260714-5046135
chat_name: server — W1 API-P8 Sync-Once Race Fix

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: FIX-SRV-API-P8-SUBMISSION-RACE

CANDIDATE:
pre_fix_code_bearing_sha: be2b1c16fa6c4919d446b76b1f15dca5767b2482
review_report_blob: 04ba2d51a041b57c677a2a742d56719a181eaf67
review_status: CLEAN_NEEDS_FIX
pre_fix_ci_run_id: 29321038276

SUMMARY:
Corrected POST sync-once so stale Busy, NotStarted, Cancelling or Shutdown snapshots cannot bypass the authoritative Worktree manual submit result. Snapshot-only prechecks remain limited to Server-specific Failed and mode/manual Unavailable states. All other categories invoke one bounded submit exactly once, map its typed result, drop an Accepted ticket without polling, and return immediately.

CHANGED_PATHS:
- crates/haze-sync-server/src/worktree_http.rs
- crates/haze-sync-server/control/report.md

FINAL_CODE_BEARING_SHA:
50461354c18ddc4d2e47202d9303b4358a27ee45

AUTHORITATIVE_SUBMIT_FLOW:
1. Upgrade the weak host reference once; failed upgrade returns Unavailable.
2. Read one passive host snapshot.
3. Return snapshot-only Failed for host failure.
4. Return snapshot-only Unavailable for disabled/non-manual modes.
5. For Available, Busy, NotStarted, Cancelling and Shutdown snapshot categories, construct exactly one WorktreeRuntimeManualRequest::dry_run from the stored validated action budget.
6. Call ServerWorktreeRuntimeHost::submit_manual exactly once.
7. Map only the typed WorktreeRuntimeManualSubmission result:
   - Accepted -> Accepted
   - Busy -> Busy
   - NotStarted -> NotStarted
   - Cancelling -> Cancelling
   - Shutdown -> Shutdown
8. Drop the Accepted ticket without polling or waiting; no completion semantics are inferred.

RACE_RESOLUTION:
- stale Busy snapshot can now produce authoritative Accepted when the owner gate becomes available before submit;
- stale NotStarted/Cancelling/Shutdown observations cannot be returned without typed submit evaluation;
- snapshot-to-submit races are resolved by the accepted Worktree shared gate;
- no retry, second submit, probe request, completion wait or completion poll exists.

FOCUSED_TESTS:
- stale Busy snapshot invokes submit once and accepts a later authoritative Accepted result;
- NotStarted, Cancelling and Shutdown snapshots invoke submit once rather than bypassing it;
- authoritative Busy, Cancelling and Shutdown results are preserved;
- each submit-capable path invokes the injected submit boundary exactly once;
- Failed and Unavailable remain snapshot-only and invoke submit zero times;
- the test boundary is synchronous FnOnce, proving no retry/poll/wait behavior is introduced.

OWNERSHIP_AND_SCOPE:
weak_host_ownership_preserved: yes
unique_shutdown_join_ownership_preserved: yes
new_task_runtime_watcher_or_poller: no
mutex_or_duplicate_gate_added: no
api_p8_blobs_modified: no
worktree_runtime_gate_watcher_executor_modified: no
storage_core_cli_deployment_modified: no
migrations_or_workflows_modified: no
public_http_mapping_changed: no

CI_DIAGNOSTIC_ITERATION:
run_id: 29326359943
run_number: 1939
head_sha: ac24af92fa8a43ed482407784a37564032722665
product_checks:
- cargo check: success
- cargo test: success
- cargo clippy: success
diagnostics_failure: rustfmt-only
artifact_id: 8308117911
resolution: applied the exact single rustfmt diff without behavior change

AUTHORITATIVE_CI:
workflow: Component CI
run_id: 29326558901
run_number: 1940
run_attempt: 1
head_sha: 50461354c18ddc4d2e47202d9303b4358a27ee45
conclusion: success
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
diagnostics_upload: skipped_expected_no_failure
db_capable: yes

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer

NEXT_GATE:
Perform final focused Server review of exact SHA 50461354c18ddc4d2e47202d9303b4358a27ee45. CLI-P6A remains blocked until CLEAN_ACCEPT. Do not begin CLI or Deployment work in this phase.

FINAL_VERDICT:
FIX_COMPLETE. POST sync-once now uses snapshot only for Failed/Unavailable Server conditions and uses exactly one authoritative typed Worktree submission for all race-sensitive lifecycle and Busy states. Focused tests pass and DB-capable Component CI run 29326558901 succeeds on exact final SHA 50461354c18ddc4d2e47202d9303b4358a27ee45.

PUSHED:
yes
