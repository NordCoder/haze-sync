REPORT_TYPE:
IMPLEMENTATION

STATUS:
BLOCKED_BY_CONTRACT

AGENT:
role: implementation-worker
agent_execution_id: server-srv-p7b4-contract-audit-20260713
chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-P7B4-HOSTED-WORKTREE-RUNTIME

ACCEPTED_BASELINE:
server_bounded_executor_sha: f8475af72b3e1795c5b11fa39f4625191eff59b1
server_executor_clean_report_commit: b9e22533091a9477876b91729c04682266a1b718
worktree_owner_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
storage_owner_sha: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
server_application_services_sha: 647dce7b624d67663632808906896cb6745ea7e7

SUMMARY:
Performed the mandatory hosted-runtime feasibility audit before changing Server product code. The accepted Worktree runtime contract is sufficient for host-driven automatic polling with a host-supplied clock, watcher and executor, but it is not sufficient to implement the required production hosted runtime without Server taking ownership of Worktree filesystem-watcher and manual-cycle scheduler semantics.

CONTRACT_BLOCKERS:
1. No production WorktreeWatcher implementation exists in the accepted Worktree snapshot.
   - accepted file: crates/haze-sync-worktree/src/runtime.rs
   - accepted blob: e8ea318c214dde09b01b4c2aa903e5a0f61766fb
   - the file defines only the WorktreeWatcher trait with start, poll_hint and shutdown.
   - accepted export file: crates/haze-sync-worktree/src/lib.rs
   - accepted blob: f3be9ddac7f4fbec28ad75f803f2d2ab2ea09a52
   - no concrete watcher type is exported.
   - accepted tests: crates/haze-sync-worktree/src/runtime_tests.rs
   - accepted blob: 2ca0803a247f172aa63584a4f0f7c1a0824eb181
   - tests use a test-only FakeWatcher; repository search found no production `impl WorktreeWatcher for ...`.

2. WorktreeRuntimeService has no manual-cycle execution entrypoint.
   - accepted file: crates/haze-sync-worktree/src/runtime.rs
   - WorktreeRuntimeCycleRequest::manual_dry_run constructs a request only.
   - WorktreeRuntimeService::poll drives startup, periodic and watcher cycles only.
   - WorktreeRuntimeService exposes executor_mut(), but directly calling executor_mut().run_cycle(...) from Server would bypass Worktree-owned cycle accounting, last-cycle status, lifecycle/cancellation transitions and scheduler no-overlap semantics.
   - implementing duplicate manual busy/coalesced/status accounting in Server would violate the accepted ownership boundary that Worktree owns scheduler policy and no-overlap semantics.

WHY_SERVER_CANNOT_SAFELY_WORK_AROUND:
- Implementing a filesystem watcher in Server would move Worktree-owned watcher/filesystem semantics into the consumer component.
- Supplying a channel-only watcher with no accepted filesystem producer would make watcher-trigger behavior non-production and would not satisfy the requested hosted runtime honestly.
- Calling the executor directly for manual DryRun would bypass WorktreeRuntimeService and duplicate scheduler/status behavior in Server.
- Modifying accepted Worktree source is explicitly forbidden in this Server slot.
- No nested runtime, block_on, detached task, fake production watcher or fabricated manual accounting was introduced.

REQUIRED_OWNER_CHANGE:
Route a bounded Worktree owner phase that adds:
- a production path-free WorktreeWatcher implementation or an accepted Worktree-owned watcher factory/channel boundary with lifecycle semantics;
- a WorktreeRuntimeService manual-cycle method that accepts an explicit WorktreeRuntimeCycleRequest or dedicated manual DryRun request, preserves at-most-one-cycle behavior, updates accepted counters/last-cycle/lifecycle state and rejects busy/cancelling/shutdown states with typed coarse outcomes;
- focused tests for manual busy rejection, manual DryRun accounting, cancellation and watcher lifecycle;
- normal Worktree Component CI and mandatory clean-code review.

CHANGED_FILES:
- crates/haze-sync-server/control/report.md

PRODUCT_CODE_CHANGED:
no

PROTECTED_SCOPE:
accepted_worktree_snapshot_modified: no
accepted_storage_snapshot_modified: no
migrations_modified: no
core_api_cli_deployment_modified: no
workflows_modified: no
sibling_control_modified: no

TESTS_AND_CI:
checks_run:
- live PR/control bootstrap
- accepted Worktree runtime contract inspection at exact owner SHA
- accepted Worktree exports inspection
- accepted Worktree runtime tests inspection
- repository search for production WorktreeWatcher implementations and manual_dry_run consumers
code_bearing_ci_required: no; no product/test/dependency/workflow code was changed
ci_skip_reason: report-only contract blocker commit

SECRECY_AND_SAFETY:
secrets_committed: no
unsafe_public_output: no
raw_paths_or_database_urls_exposed: no
background_tasks_added: no
provider_calls_added: no
hard_delete_added: no

BLOCKERS:
- Worktree owner contract lacks a production watcher implementation/factory.
- Worktree owner contract lacks a scheduler-accounted manual-cycle entrypoint.

NEXT_RECOMMENDED_AGENT:
orchestrator

NEXT_GATE:
Orchestrator should route the exact Worktree owner contract extension before reactivating SRV-P7B4. API-P8 and SRV-P7B5 remain blocked. SRV-P7B4 is not active beyond this completed blocker report.

FINAL_VERDICT:
BLOCKED_BY_CONTRACT. A correct ServerWorktreeRuntimeHost cannot be implemented from the accepted immutable Worktree snapshot without either fabricating a non-production watcher or bypassing and duplicating Worktree-owned scheduler/manual-cycle semantics. No workaround was committed.

PUSHED:
yes
