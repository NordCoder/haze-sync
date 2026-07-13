REPORT_TYPE: CLEAN_CODE_REVIEW

STATUS: CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
chat_name: worktree — W1 WT-P11 Final Functional Review

COMPONENT:
name: worktree
path: crates/haze-sync-worktree
branch: component/worktree
control_prompt_path: crates/haze-sync-worktree/control/prompt.md
control_report_path: crates/haze-sync-worktree/control/report.md

WAVE:
id: W1
phase_id: WT-P11-FINAL-FUNCTIONAL-REVIEW

REVIEW_TARGET:
accepted_baseline_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
prior_reviewed_sha: 61c24544a3fb9d785cb95ab2016f6d29f4661d3e
reviewed_code_bearing_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
fix_report_commit: 4dfa340ccce6169eab566e93be4c12747551f0fe
post_candidate_changes: report-only before this review; no later product/tooling commit invalidated the candidate

REVIEW_POLICY:
formatting_and_style_in_scope: no
functional_safety_lifecycle_concurrency_only: yes

SUMMARY:
The corrected candidate resolves validated-root construction, deterministic watcher degradation coverage, externally reachable Busy responses, and automatic/manual compatibility on completed executions. However, one substantive cancellation-safety defect remains in the hosted runtime boundary: dropping an in-flight poll future can permanently leave the shared busy gate set and close a manual ticket without a typed cancellation result. This violates pending-cycle cancellation, request lifecycle and recoverable no-overlap requirements. No code was changed during this review.

CHANGED_FILES:
- crates/haze-sync-worktree/control/report.md

FUNCTIONAL_FINDING:
severity: HIGH
area: hosted runtime cancellation and request lifecycle

problem:
- `WorktreeHostedRuntime::poll()` sets or relies on `gate.busy = true` before awaiting automatic or manual execution.
- The gate is reset only by explicit statements after the awaited future returns.
- Rust futures are cancellation-by-drop. If the host drops the `poll()` future while `service.poll()` or `service.run_manual_cycle()` is pending, execution never reaches the reset statement.
- In the manual branch, dropping the future also drops the moved response sender, so the accepted ticket becomes `Closed` rather than receiving a typed `Cancelled` outcome.

exact_evidence:
- manual branch awaits `run_manual_cycle` and resets busy only afterward;
- automatic branch awaits `service.poll` and resets busy only afterward;
- no RAII guard owns the busy reset;
- no Drop implementation or in-flight request state restores the gate or completes the ticket;
- focused tests use immediately-ready cycle futures and `block_on` every hosted poll to completion; no pending future is polled once and dropped.

impact:
- After a dropped automatic poll future, all subsequent manual submissions can return `Busy` indefinitely.
- After a dropped manual poll future, the accepted ticket can become `Closed`, the gate can remain permanently busy, and no coarse typed cancellation result is delivered.
- The hosted boundary is therefore not cancellation-safe under normal async host behavior such as select cancellation, timeout, task shutdown or dropped request processing.
- The runtime service remains allocated and may report Running while its host-facing scheduler boundary is wedged.

required_correction:
- Introduce an RAII in-flight/busy guard whose Drop always clears the shared busy gate.
- For a dequeued manual request, ensure cancellation-by-drop completes the ticket with `WorktreeRuntimeManualOutcome::Failed(WorktreeRuntimeCycleFailure::Cancelled)` or another existing approved coarse typed cancellation outcome.
- Do not commit accounting as a completed cycle when the future is dropped before executor completion.
- Add a deterministic pending executor future test that:
  - submits and accepts a manual request;
  - polls hosted `poll()` once to Pending;
  - drops that future;
  - verifies the ticket receives the typed cancellation result rather than remaining Pending/Closed;
  - verifies a subsequent request is accepted rather than permanently Busy;
  - verifies cycle counters and last-cycle fields remain internally consistent.
- Add the equivalent dropped automatic-poll test proving the busy gate is released and later manual/automatic work remains schedulable.

VERIFIED_CORRECTIONS_AND_POSITIVE_FINDINGS:
- Production watcher construction is enforced through `&WorktreeConfig`; arbitrary absolute PathBuf construction is removed.
- Watcher hints and Debug remain path-free/root-redacted.
- Internal deterministic seams cover bounded overflow, backend failure, closure and repeated shutdown.
- The manual handle makes NotStarted, Busy, Cancelling and Shutdown externally reachable without exposing the executor.
- The request channel is bounded and the service remains sole owner of executor, cancellation, counters and last-cycle status.
- Completed manual cycles preserve startup pending state and do not consume periodic or watcher scheduling state.
- Automatic startup, periodic and watcher cycles remain covered and green.
- No detached task, thread, hidden runtime, raw path, sibling component or workflow change was introduced.

TESTS_AND_CI:
authoritative_run_id: 29269422217
authoritative_run_number: 1876
head_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
conclusion: success
ci_interpretation: green CI validates current tests but does not cover cancellation-by-drop defect

CORRECTIONS_MADE:
none

BLOCKERS:
- Hosted runtime busy/ticket lifecycle is not cancellation-safe when an in-flight poll future is dropped.

NEXT_RECOMMENDED_AGENT:
fixer-worker with a narrowly scoped hosted-runtime cancellation fix and deterministic pending-future tests

SERVER_FAN_IN:
may_begin: no
reason: final functional review is CLEAN_NEEDS_FIX

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Formatting and stylistic matters were explicitly excluded from review scope. The sole blocker is a concrete functional async-cancellation defect in `WorktreeHostedRuntime::poll()`. Correct it, obtain exact-SHA green Component CI, and repeat final functional review before Server fan-in.

PUSHED:
yes
