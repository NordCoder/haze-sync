REPORT_TYPE: FIX

STATUS: FIX_COMPLETE

AGENT:
role: fixer-worker
chat_name: worktree — W1 WT-P11 Cancellation Safety Fix

COMPONENT:
name: worktree
path: crates/haze-sync-worktree
branch: component/worktree

WAVE:
id: W1
phase_id: WT-P11-CANCELLATION-FIX

REVIEWED_CANDIDATE:
code_bearing_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
functional_review_report_commit: 7e761ccf1b3ba0036d10067a9d6e4bf395db4bf1
review_status: CLEAN_NEEDS_FIX
sole_blocker: cancellation-by-drop could wedge the hosted busy gate and close a manual ticket without typed cancellation

SUMMARY:
Implemented cancellation-by-drop safety at both the hosted scheduler boundary and the internal runtime cycle boundary. Dropped pending futures now release all in-flight flags, manual tickets receive the existing typed Cancelled outcome, incomplete attempts do not commit counters or last-cycle cause/result, and automatic scheduling state remains pending for later execution.

CHANGED_FILES:
- crates/haze-sync-worktree/src/hosted_runtime.rs
- crates/haze-sync-worktree/src/runtime.rs
- crates/haze-sync-worktree/src/wt_p11_tests.rs
- crates/haze-sync-worktree/control/report.md

RAII_AND_TICKET_SEMANTICS:
- Added HostedInFlightGuard owning the shared busy gate.
- Normal manual completion sends the actual outcome and releases busy.
- Dropping a pending manual hosted poll sends WorktreeRuntimeManualOutcome::Failed(WorktreeRuntimeCycleFailure::Cancelled) and releases busy.
- Dropping a pending automatic hosted poll releases busy.
- Added CycleAttemptGuard inside WorktreeRuntimeService.
- CycleAttemptGuard clears cycle_in_progress on normal completion and Drop.
- On Drop before completion it restores the previous last_cycle_cause.
- Incomplete dropped attempts do not increment completed/failed counters and do not write last_cycle.
- Automatic startup/watcher/periodic state is advanced only after completed execution, so cancellation-by-drop preserves pending work.

DETERMINISTIC_TESTS:
- accepted manual request reaches Pending;
- hosted poll future is dropped;
- ticket receives typed Cancelled instead of Pending/Closed;
- busy and cycle_in_progress are released;
- counters, last_cycle_cause and last_cycle remain unchanged;
- a subsequent manual request is accepted and completes;
- dropped automatic startup poll releases busy;
- startup remains pending and later completes after intervening manual work.

CI_DIAGNOSTICS:
initial_run:
- run_id: 29272665774
- run_number: 1879
- head_sha: 8b0335071af4e275b6fa9762974ab6dd2dbe5256
- cargo_fmt/check/test/clippy: success
- diagnostics_finalizer: failure
- artifact_id: 8288107031
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29272665774__attempt-1
- artifact_files_read: summary.md, manifest.json, failures/rust-fmt.txt, logs/rust-fmt.log
- root_cause: two rustfmt-only diffs in hosted_runtime.rs and runtime.rs

FINAL_CANDIDATE:
code_bearing_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b

FINAL_CI:
workflow: Component CI
run_id: 29272964159
run_number: 1881
head_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
conclusion: success

SCOPE_AND_SAFETY:
allowed_files_only: yes
server_changes: none
sibling_component_changes: none
workflow_changes: none
detached_tasks_or_hidden_runtime: none
raw_paths_or_backend_errors_exposed: no
ci_skip_used: report-only commit only

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer for final lightweight functional verification

SERVER_FAN_IN:
may_begin: no
reason: final lightweight functional verification is still required

FINAL_VERDICT:
FIX_COMPLETE. Cancellation-by-drop is safe at exact code-bearing SHA b38264ce2b09632a4c0bab0dd77319e1db239a3b with authoritative green Component CI run 29272964159. Do not claim CLEAN_ACCEPT or begin Server fan-in until the next control slot.

PUSHED:
yes
