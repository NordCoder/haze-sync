REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: server-srv-p7b4-functional-review-20260713-5536d26
chat_name: server — W1 SRV-P7B4 Hosted Runtime Functional Review

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-P7B4-FUNCTIONAL-REVIEW

CANDIDATE:
accepted_integration_baseline: 1b2b572a1a200f2968d005e48e9c0674f9db8bc0
final_code_bearing_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
implementation_report_commit: 896f28c3c1d5d8f86d4b58e3428b1d289ee6aa8a
implementation_report_blob: c9fa7ce4ae3876b8d602fbf15662a3651babc023

REVIEW_SCOPE:
Functional lifecycle, task ownership/join, shutdown, modes, manual boundary, secrecy, tests, exact CI and protected scope. Formatting, naming taste and style were excluded as required.

POSITIVE_FINDINGS:
- Enabled composition uses one retained Tokio JoinHandle and one oneshot shutdown sender.
- Disabled mode is inert and task-free.
- No nested runtime, block_on, internal HTTP or explicit second executor path was introduced.
- Durable root binding is attempted before task spawn using a coarse secret-safe error.
- ProductionWorktreeWatcher, WorktreeHostedRuntime and ServerWorktreeCycleExecutor are composed through accepted boundaries.
- Manual submission uses WorktreeRuntimeManualHandle and returns accepted typed submission outcomes.
- Explicit shutdown uses a bounded timeout; timeout aborts and then awaits the retained JoinHandle.
- Internal status and error rendering are coarse and path/URL/payload-free.
- No accepted Worktree/Storage, migration, Core/API/CLI/Deployment, public route/DTO, readiness or workflow scope was modified by the product candidate.

SUBSTANTIVE_FINDINGS:
1. Enabled startup does not establish a successful hosted-runtime start before returning.
   - ServerWorktreeRuntimeHost::start constructs WorktreeHostedRuntime and immediately returns Self::spawn_runtime(...).
   - spawn_runtime calls tokio::spawn(run_hosted(...)) and returns the host without a startup acknowledgement channel.
   - WorktreeHostedRuntime::start, including ProductionWorktreeWatcher::start, executes later inside run_hosted.
   - Therefore watcher/runtime startup failure is not returned through ServerWorktreeRuntimeHost::start or Server startup. The HTTP listener can be bound and served while the Worktree task has already failed.
   - On this early failure path run_hosted returns RuntimeFailed before publishing a Failed status, leaving the shared host status at Starting until a later shutdown/join observes the task result.
   - This violates fail-closed enabled startup and produces stale internal status for an actual task failure.

2. Host-owned lifecycle behavior is not substantively tested.
   - The only tests in crates/haze-sync-server/src/worktree_host.rs cover disabled/bounded defaults, one invalid config aggregate, Disabled inert shutdown, and error redaction.
   - No Server-host test constructs the generic spawn_runtime/run_hosted path with fake clock/watcher/executor.
   - No Server-host test verifies successful startup acknowledgement, startup failure propagation, periodic polling, watcher/debounce forwarding, accepted/completed manual flow, Busy, DryRun request behavior, no overlap, cooperative cancellation, shutdown during an in-flight poll, timeout abort-and-await, task failure status, or mandatory join completion.
   - Accepted lower-level Worktree tests cannot prove Server-owned task orchestration, startup propagation, timeout handling or join behavior. The active prompt explicitly requires host-owned lifecycle coverage rather than relying only on lower-level Worktree tests.

REQUIRED_FIX:
- Add a bounded startup acknowledgement from the hosted task to ServerWorktreeRuntimeHost::start.
- Do not return an enabled host until WorktreeHostedRuntime::start has succeeded and initial status is published.
- Propagate watcher/runtime startup failure as coarse ServerWorktreeHostError and join/clean up the failed task before returning.
- Publish Failed status on every task error path, including failure during runtime.start.
- Add focused Server-owned Tokio tests using fake Worktree clock/watcher/executor and the real Server host loop for:
  - enabled successful startup acknowledgement;
  - startup failure propagation and no leaked task;
  - startup/periodic/watcher/manual polling through the host;
  - manual accepted/completed and Busy behavior;
  - no overlapping cycles;
  - cooperative cancellation and shutdown during in-flight work;
  - bounded timeout abort-and-await;
  - mandatory join and final Shutdown/Failed status;
  - coarse redacted status/errors.
- Preserve accepted Worktree/Storage source and all protected scope.

CI:
workflow: Component CI
run_id: 29278756276
run_number: 1888
run_attempt: 1
head_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
conclusion: success
db_capable: yes
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
ci_assessment: green but insufficient to accept because the lifecycle defect and missing host-owned tests are not exercised

LATER_COMMIT_VALIDATION:
Compared 5536d260bb4f95ec11c0cd07501c23903a72757d..842b1549936537de6875a70a0238b943cc49886b. All later changes before this report are confined to crates/haze-sync-server/control prompt/state/log files. No later product or tooling commit invalidated or corrected the candidate.

CODE_CHANGES_DURING_REVIEW:
none

PROTECTED_SCOPE:
accepted_worktree_modified: no
accepted_storage_modified: no
migrations_or_schema_modified: no
core_api_cli_deployment_modified: no
public_routes_dtos_or_readiness_modified: no
workflows_modified: no
sibling_branches_modified: no
pr_merged_or_draft_state_changed: no

BLOCKERS:
- enabled hosted-runtime startup failure is asynchronous and not propagated before Server begins serving
- shared status can remain Starting after task startup failure
- required Server-host lifecycle/join/concurrency test coverage is absent

NEXT_RECOMMENDED_AGENT:
fixer-worker

NEXT_GATE:
Route a focused SRV-P7B4 lifecycle fix. API-P8 and SRV-P7B5 remain blocked. A new exact code-bearing SHA must pass authoritative DB-capable Component CI and then receive a focused functional clean review.

FINAL_VERDICT:
CLEAN_NEEDS_FIX. The composition boundaries and exact-SHA CI are otherwise sound, but enabled startup is not fail-closed because runtime/watcher start occurs after the host is returned, and the Server-owned task lifecycle is not substantively tested. SRV-P7B4 is not accepted.

PUSHED:
yes
