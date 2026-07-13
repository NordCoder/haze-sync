REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: server-srv-p7b4-lifecycle-fix-20260713-55ed0d6
chat_name: server — W1 SRV-P7B4 Lifecycle Fix

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-P7B4-LIFECYCLE-FIX

REVIEWED_CANDIDATE:
code_bearing_sha: 5536d260bb4f95ec11c0cd07501c23903a72757d
functional_review_report_commit: de06e35bb687448ea61fe6eacc0d1dd76ad04766
functional_review_report_blob: 07e298a0622c6e5b1210160dc70b04c714a858e7
review_status: CLEAN_NEEDS_FIX

SUMMARY:
Corrected fail-open enabled startup and added Server-owned lifecycle/concurrency/join coverage around the real generic host loop. Enabled startup now waits for a bounded task acknowledgement after the accepted Worktree runtime has started, the typed watcher state is verified, and initial Running status is published. Any startup error or timeout is returned coarsely only after the retained task is joined or abort-and-awaited.

CHANGED_PRODUCT_AND_TEST_PATHS:
- crates/haze-sync-server/src/worktree_host.rs
- crates/haze-sync-server/src/worktree_host_tests.rs

FINAL_CODE_BEARING_SHA:
55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe

STARTUP_ACKNOWLEDGEMENT:
- ServerWorktreeRuntimeHost::start_runtime creates a bounded oneshot acknowledgement channel before spawning the single hosted task.
- The caller waits with the configured bounded timeout.
- run_hosted invokes WorktreeHostedRuntime::start inside the owned task.
- Successful acknowledgement is sent only after initial status has been published as Running.
- The host is not returned before acknowledgement success.
- Disabled remains inert, task-free and acknowledgement-free.

TYPED WATCHER_FAILURE_HANDLING:
- The accepted Worktree service intentionally returns a start summary even when watcher startup fails, encoding failure in WorktreeRuntimeWatcherState::Failed.
- The Server host now checks that typed accepted state after WorktreeHostedRuntime::start.
- Lifecycle error or typed watcher failure publishes Failed, sends coarse RuntimeFailed and exits the task.
- No raw notify, path, I/O or backend error is exposed.

FAILED_START_CLEANUP:
- Startup error, acknowledgement channel closure and startup timeout all invoke bounded failed-start join cleanup.
- If the task does not join within the configured timeout, it is aborted and then awaited.
- Startup timeout first signals shutdown when possible, then joins or abort-and-awaits.
- No failed-start path returns a retained or detached task.

STATUS_TRANSITIONS:
- Initial shared state is Starting.
- Successful startup publishes Running before acknowledgement.
- Runtime lifecycle, watcher-start, poll and shutdown failures publish Failed.
- Successful explicit shutdown publishes Shutdown.
- Shutdown timeout publishes Failed after abort-and-await.

TASK_OWNERSHIP_AND_CANCELLATION:
- Exactly one retained Tokio JoinHandle remains the enabled host task.
- No detached task, nested runtime, block_on, internal HTTP or second executor path was introduced.
- Shutdown during pending poll drops the accepted poll future, then requests accepted cooperative cancellation and shuts down the runtime.
- Cancellation-by-drop releases the accepted in-flight/no-overlap guard.
- Explicit shutdown remains bounded and mandatory-join.

SERVER_OWNED_TESTS:
Added focused Tokio tests using the real Server start_runtime/run_hosted path with narrow fake Worktree clock/watcher/executor implementations for:
- Disabled inert and join-free behavior;
- enabled startup acknowledgement and initial Running status;
- typed watcher startup failure propagation;
- failed-start task cleanup with watcher drop evidence;
- startup, periodic and watcher/debounce cycles through the Server host;
- manual accepted/completed flow through the host boundary;
- typed Busy response while a manual cycle is pending;
- DryRun full-scan request and non-mutating mode semantics;
- at-most-one active cycle across triggers;
- pending-cycle cancellation-by-drop during shutdown;
- mandatory joined Shutdown result;
- bounded shutdown timeout with abort-and-await and Failed status;
- coarse redacted status and errors.

DIAGNOSTIC_ITERATIONS:
- run 29281023841 on SHA 9a2c45f221b3090197b0ade3015cdb97863f7511 identified test expectation issues plus rustfmt. Production compile/clippy remained green.
- run 29281259376 on SHA 0970cf19538b9a1e8c7a1f7cc78aed9845ec171f showed two remaining test-model issues: accepted watcher failure is typed in status rather than returned as lifecycle Err, and periodic deadlines masked watcher debounce. Both were corrected without changing owner contracts.
- Final tests use ImportOnly for watcher startup failure and separated periodic/watcher clock deadlines.

AUTHORITATIVE_CI:
workflow: Component CI
run_id: 29281569666
run_number: 1893
run_attempt: 1
head_sha: 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe
conclusion: success
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
diagnostics_upload: skipped_expected_no_failure
ci_status: CI_GREEN_DB_VERIFIED

PROTECTED_SCOPE:
accepted_worktree_source_modified: no
accepted_storage_source_modified: no
migrations_or_schema_modified: no
core_api_cli_deployment_modified: no
public_routes_dtos_or_readiness_modified: no
workflows_modified: no
sibling_branches_modified: no
pr_merged_or_draft_state_changed: no
history_rewritten: no

SECRECY:
paths_or_root_fingerprints_exposed: no
database_urls_or_sqlx_errors_exposed: no
notify_or_io_errors_exposed: no
payloads_tokens_or_idempotency_material_exposed: no

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
clean-code-reviewer

NEXT_GATE:
Final focused functional verification of exact SHA 55ed0d6c6ab9b78a953b954fcf5a9a68a6a708fe is required. SRV-P7B5 and API-P8 remain blocked until SRV-P7B4 receives CLEAN_ACCEPT.

FINAL_VERDICT:
FIX_COMPLETE. Enabled startup is now bounded and fail-closed, typed watcher startup failure is propagated coarsely, failed startup cannot leak or detach a task, all error paths publish Failed, and Server-owned lifecycle/manual/no-overlap/cancellation/timeout/join tests pass authoritative DB-capable Component CI run 29281569666 on the exact final code-bearing SHA.

PUSHED:
yes
