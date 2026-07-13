REPORT_TYPE: CLEAN_CODE_REVIEW

STATUS: CLEAN_ACCEPT

AGENT:
role: clean-code-reviewer
chat_name: worktree — W1 WT-P12 Manual Status Review

COMPONENT:
name: worktree
path: crates/haze-sync-worktree
branch: component/worktree

WAVE:
id: W1
phase_id: WT-P12-FUNCTIONAL-REVIEW

REVIEW_SCOPE:
accepted_wt_p11_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
reviewed_wt_p12_code_bearing_sha: 526714cdfe185713a09af68fd5bddcb967a7902e
implementation_report_commit: f7ad06d4ddec016c35555388634926bda5f963b0
formatting_and_style_in_scope: no

SUMMARY:
The WT-P12 passive manual status extension is functionally accepted. It reads the same Worktree-owned Gate used by manual submission, automatic/manual no-overlap, completion, cancellation-by-drop and lifecycle transitions. It adds no mirror, probe, generation approximation, task, poller, runtime, watcher or retry. The exposed contract is coarse, passive and secret-safe, and the exact reviewed SHA has authoritative green Component CI.

AUTHORITATIVE_SHARED_STATE:
- `WorktreeRuntimeManualStatusHandle` owns only an `Arc<Gate>`.
- The Gate is the existing accepted gate containing the lifecycle AtomicU8 and shared busy AtomicBool.
- Submission, hosted execution guards, cancellation Drop paths, start, request_cancel and shutdown all mutate this same Gate.
- Status reads perform only bounded synchronous Acquire loads from that Gate.
- No duplicated state or downstream-style mirror exists.

STATUS_CONTRACT:
fields:
- lifecycle: WorktreeRuntimeLifecycle
- busy: bool

precise_busy_meaning:
- `busy == true` means the authoritative shared hosted-runtime no-overlap gate is claimed and a new manual submission cannot currently be accepted.
- The gate may be claimed by an accepted queued/executing manual request or by a host-driven automatic poll/cycle.
- This is intentionally broader than “a manual request is executing”; it is exact manual availability, not an approximation.
- A short automatic poll, including a DryRun/idle-style poll, may therefore report busy while it owns the gate; a concurrent manual submit would also receive Busy during that same interval, so the status is not falsely reporting availability.

VERIFIED_TRANSITIONS:
- Created starts with lifecycle Created and busy false.
- Successful start publishes Running.
- Accepted submission claims busy before returning Accepted.
- Busy rejection leaves the already-claimed gate true.
- Busy remains true while a manual request is queued, pending or executing.
- Normal completion sends the result and releases busy through the existing in-flight guard.
- Cancellation-by-drop sends typed `Failed(Cancelled)` and releases busy through the same RAII owner path.
- request_cancel publishes Cancelling.
- shutdown publishes Shutdown and releases busy.
- Dropping status handles is passive.
- Dropping tickets alone does not mutate the Gate.

RACE_AND_IDENTITY_SAFETY:
- A second request cannot become accepted until the existing owner releases the single gate.
- An old completed ticket contains only its result receiver and has no capability to mutate Gate state.
- Dropping an old ticket therefore cannot clear a newer accepted request.
- Dropping a newer ticket also does not clear its queued/executing ownership; the hosted guard remains the state owner.
- Completion and cancellation release are performed by the current in-flight guard, not by arbitrary status or ticket objects.

PASSIVITY_AND_SECRECY:
- no submission method exists on the status handle;
- no polling, I/O, filesystem operation or provider call occurs during status reads;
- no request payload, identity, path, token, cursor, backend detail or error payload is exposed;
- Debug output contains only the coarse status;
- reads are synchronous, lock-free, bounded and side-effect free.

REGRESSION_SAFETY:
- Existing WT-P11 cancellation-by-drop guards are unchanged in behavior.
- Existing automatic/manual shared-gate no-overlap behavior remains intact.
- Existing scheduling and accounting tests remain green.
- Focused WT-P12 tests cover lifecycle, immediate accepted busy, Busy preservation, pending execution, completion, cancellation-by-drop, old/new ticket drops and passive reads.

POST_CANDIDATE_VALIDATION:
comparison: 526714cdfe185713a09af68fd5bddcb967a7902e..component/worktree
result: only Worktree control prompt/state/log files changed
product_or_tooling_invalidation: none

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

SCOPE:
server_changes: none
storage_core_api_cli_deployment_changes: none
migration_or_schema_changes: none
workflow_changes: none
public_http_or_dto_changes: none
scope_expansion: none

CORRECTIONS_MADE:
none; review-only phase

BLOCKERS:
none

SERVER_FAN_IN:
may_begin: yes
required_sha: 526714cdfe185713a09af68fd5bddcb967a7902e
condition: Orchestrator may perform or assign exact-SHA fan-in of the accepted WT-P12 product files to component/server and resume SRV-P7B5; this reviewer did not modify Server

FINAL_VERDICT:
CLEAN_ACCEPT. The WT-P12 authoritative passive lifecycle/busy contract at exact code-bearing SHA 526714cdfe185713a09af68fd5bddcb967a7902e satisfies shared-state ownership, lifecycle, busy, race, passivity, secrecy and protected-scope requirements with authoritative green Component CI run 29287214701. Exact-SHA Server fan-in may begin.

PUSHED:
yes
