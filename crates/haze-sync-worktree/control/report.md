REPORT_TYPE: IMPLEMENTATION

STATUS: SELF_ACCEPT

AGENT:
role: implementation-worker
chat_name: worktree — W1 WT-P12 Manual Status Contract

COMPONENT:
name: worktree
path: crates/haze-sync-worktree
branch: component/worktree

WAVE:
id: W1
phase_id: WT-P12-MANUAL-STATUS-CONTRACT

ACCEPTED_BASELINE:
wt_p11_code_bearing_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
clean_report_commit: b76f88369d079a9cb264bb4cdb9b6c62e361a9bc
clean_status: CLEAN_ACCEPT

SUMMARY:
Added the smallest passive authoritative manual lifecycle/busy contract over the existing Worktree-owned gate. The new cloneable status handle reads the same AtomicU8 lifecycle and AtomicBool busy state used by submission, execution, completion, cancellation-by-drop and shutdown. It performs no submission, polling, I/O or background work and exposes only coarse typed lifecycle and busy fields.

FINAL_CANDIDATE:
code_bearing_sha: 526714cdfe185713a09af68fd5bddcb967a7902e

CHANGED_FILES:
- crates/haze-sync-worktree/src/hosted_runtime.rs
- crates/haze-sync-worktree/src/lib.rs
- crates/haze-sync-worktree/src/wt_p12_tests.rs
- crates/haze-sync-worktree/control/report.md

AUTHORITATIVE_STATE_DESIGN:
- `Gate` remains the sole owner of manual lifecycle and busy state.
- `WorktreeRuntimeManualStatusHandle` contains only `Arc<Gate>`.
- `WorktreeRuntimeManualStatus` exposes only:
  - lifecycle: WorktreeRuntimeLifecycle
  - busy: bool
- status reads use atomic Acquire loads and are synchronous, lock-free, bounded, side-effect free and non-blocking.
- no duplicate gate, mirror, request probe, generation store, task, poller, watcher, runtime or retry was added.
- status handles can be obtained from either the submission handle or hosted runtime without changing existing constructor return shape.

LIFECYCLE_AND_BUSY_TRANSITIONS:
- Created: lifecycle Created, busy false.
- start: lifecycle atomically transitions to Running; idle remains not busy.
- accepted submission: existing compare_exchange sets busy true before enqueue returns Accepted.
- Busy rejection: does not mutate the already-true gate.
- dequeue/execution: busy remains true through pending executor work.
- normal manual completion: existing guard sends outcome then clears busy.
- cancellation-by-drop: existing RAII guard sends typed Cancelled and clears busy.
- automatic execution uses the same busy gate and releases it on completion/drop.
- request_cancel: lifecycle transitions to Cancelling.
- shutdown: lifecycle transitions to Shutdown and busy is cleared.
- dropping status handles or tickets does not mutate gate state.

TESTS:
- Created/not-started lifecycle and not busy.
- Running idle and not busy.
- accepted request becomes busy immediately.
- Busy rejection preserves busy.
- pending/executing request remains busy.
- normal completion clears busy.
- cancellation-by-drop clears busy and ticket receives typed Cancelled.
- dropping an older completed ticket cannot clear a newer accepted request.
- dropping a newer ticket alone does not clear authoritative busy while queued/executing work remains owned by Worktree.
- Cancelling and Shutdown lifecycle transitions are visible.
- repeated passive reads are side-effect free.
- Debug output contains only coarse lifecycle/busy status and no request/path/token details.
- all existing WT-P11 scheduling, cancellation, no-overlap and accounting tests remain green.

CI_DIAGNOSTICS:
initial_run:
- run_id: 29287039327
- run_number: 1904
- head_sha: fa1d414c62007ef703d96dd43cd3ff405b5f3dba
- cargo_fmt/check/test/clippy: success
- diagnostics_finalizer: failure
- artifact_id: 8293603314
- artifact_files_read: summary.md, manifest.json, failures/rust-fmt.txt, logs/rust-fmt.log
- root_cause: two rustfmt-only diffs in wt_p12_tests.rs

AUTHORITATIVE_CI:
workflow: Component CI
run_id: 29287214701
run_number: 1905
head_sha: 526714cdfe185713a09af68fd5bddcb967a7902e
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
workflow_or_migration_changes: none
public_http_or_dto_changes: none
payload_path_backend_or_secret_detail_exposed: no
new_task_poller_runtime_or_retry: none
ci_skip_used: report-only commit only

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer for focused WT-P12 functional review

SERVER_FAN_IN:
may_begin: no
reason: focused functional clean review and exact-SHA acceptance are still required

FINAL_VERDICT:
SELF_ACCEPT. WT-P12 adds an authoritative passive manual lifecycle/busy view over the existing Worktree gate at exact code-bearing SHA 526714cdfe185713a09af68fd5bddcb967a7902e with authoritative green Component CI run 29287214701. Do not claim CLEAN_ACCEPT or modify Server until the next control slot.

PUSHED:
yes
