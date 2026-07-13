# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B3 Bounded Worktree Executor
prompt_revision: verified by Orchestrator after exact-SHA fan-in CLEAN_ACCEPT and green DB-capable Component CI

wave: W1
phase: SRV-P7B3-BOUNDED-WORKTREE-EXECUTOR

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN
known_failed_checks: []

accepted_integration_baseline:
- fan_in_code_bearing_sha: 4f8b3d9219961409847b12e393d9a38dc6377dea
- fan_in_implementation_report_commit: ae996b4811dde20a1bf535f85f214f6f61d5b53f
- fan_in_clean_report_commit: 05250a460f9865d00a822e795ec510866f52bd0a
- fan_in_clean_report_blob: 558bd7b62ac08dbdc06efacceace84764217de70
- fan_in_clean_status: CLEAN_ACCEPT
- post_candidate_product_or_tooling_changes: none

accepted_owner_shas:
- worktree: 1942946331e8362f19907ab6ad4eb779da70fd57
- storage: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- server_application_services: 647dce7b624d67663632808906896cb6745ea7e7

accepted_ci_evidence:
- workflow: Component CI
- run_id: 29235942761
- run_number: 1840
- run_attempt: 1
- head_sha: 4f8b3d9219961409847b12e393d9a38dc6377dea
- conclusion: success
- rust_workspace_job: success
- cargo_fmt: success
- cargo_check: success
- isolated_server_postgresql_tests: success
- isolated_storage_postgresql_tests: success
- remaining_workspace_tests: success
- cargo_clippy: success
- diagnostics_finalizer: success

archived_completed_slot:
- prompt_index: crates/haze-sync-server/control/log/20260713-090500Z-W1-SRV-P7B3-FAN-IN-CLEAN-clean-code-reviewer-prompt.md
- report_index: crates/haze-sync-server/control/log/20260713-090500Z-W1-SRV-P7B3-FAN-IN-CLEAN-clean-code-reviewer-report.md
- prompt_blob: 08e4eb004ac6e85949934ec907f8c934962515c6
- report_blob: 558bd7b62ac08dbdc06efacceace84764217de70
- report_commit: 05250a460f9865d00a822e795ec510866f52bd0a

phase_goal:
- implement ServerWorktreeCycleExecutor for exactly one bounded async Worktree cycle
- use ServerApplicationServices for every authoritative mutation/read
- use accepted Storage Worktree state and exact contiguous cursor repositories
- use accepted Worktree scan/reconciliation/materialization/trash/echo primitives
- preserve Core policy authority and owner boundaries
- provide bounded cancellation-safe replay/crash behavior

allowed_scope:
- Server-owned Worktree executor module and focused Server tests
- narrow Server application visibility/helpers required by executor
- Server-local executor composition types without hosted scheduling
- Server docs, manifest/lock only when directly required
- Server control report

protected_scope:
- accepted crates/haze-sync-worktree snapshot
- accepted crates/haze-sync-storage snapshot
- migrations
- Core, API, CLI and Deployment product files
- sibling control files and workflows

forbidden_early_work:
- ServerWorktreeRuntimeHost or joined host task
- startup/shutdown host wiring
- watcher/periodic scheduling ownership
- manual request channel or public manual-cycle route
- API-P8 DTOs or status vocabulary
- SRV-P7B5 readiness/status behavior
- CLI or Deployment behavior
- schema redesign, provider behavior, hard delete or automatic destructive repair

completion_requirements:
- component/server advances with a real code-bearing executor commit
- committed crates/haze-sync-server/control/report.md exists
- report phase SRV-P7B3-BOUNDED-WORKTREE-EXECUTOR
- report chat name server — W1 SRV-P7B3 Bounded Worktree Executor
- protected owner snapshots remain unchanged
- exact final code-bearing SHA is recorded
- authoritative DB-capable Component CI is green on exact final code-bearing SHA, or report records an honest blocker/failure with exact evidence

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> mandatory SRV-P7B3 clean-code review
- red CI -> rotate a fixer slot from exact diagnostics artifact evidence
- owner contract defect -> route to exact Worktree or Storage owner without modifying owner snapshots on Server
- SRV-P7B4 remains blocked until SRV-P7B3 CLEAN_ACCEPT
