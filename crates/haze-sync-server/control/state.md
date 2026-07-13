# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime
prompt_revision: verified by Orchestrator after SRV-P7B3 bounded executor CLEAN_ACCEPT and green DB-capable exact-SHA Component CI

wave: W1
phase: SRV-P7B4-HOSTED-WORKTREE-RUNTIME

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN
known_failed_checks: []

accepted_executor_baseline:
- final_code_bearing_sha: f8475af72b3e1795c5b11fa39f4625191eff59b1
- final_clean_report_commit: b9e22533091a9477876b91729c04682266a1b718
- final_clean_report_blob: 18d6ac03c77498570b21db0768155cf31ad978d1
- final_clean_status: CLEAN_ACCEPT
- post_candidate_product_or_tooling_changes: none

accepted_owner_shas:
- worktree: 1942946331e8362f19907ab6ad4eb779da70fd57
- storage: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- server_application_services: 647dce7b624d67663632808906896cb6745ea7e7
- server_bounded_executor: f8475af72b3e1795c5b11fa39f4625191eff59b1

accepted_ci_evidence:
- workflow: Component CI
- run_id: 29257244778
- run_number: 1860
- run_attempt: 1
- head_sha: f8475af72b3e1795c5b11fa39f4625191eff59b1
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
- prompt_index: crates/haze-sync-server/control/log/20260713-155200Z-W1-SRV-P7B3-EXECUTOR-CLEAN-REVIEW-clean-code-reviewer-prompt.md
- report_index: crates/haze-sync-server/control/log/20260713-155200Z-W1-SRV-P7B3-EXECUTOR-CLEAN-REVIEW-clean-code-reviewer-report.md
- prompt_blob: e58dfbf53c5b76cc4032190abbeade6c1321b377
- report_blob: 18d6ac03c77498570b21db0768155cf31ad978d1
- report_commit: b9e22533091a9477876b91729c04682266a1b718

phase_goal:
- implement one explicit joined cancellable ServerWorktreeRuntimeHost task
- validate safe runtime-host config and disabled defaults
- bind durable adapter/root identity before first cycle
- drive startup, periodic, watcher and bounded internal manual triggers
- preserve at-most-one-cycle and exact mode behavior
- provide cooperative cancellation, bounded graceful shutdown and join
- keep status internal count/category-only and secret-safe

allowed_scope:
- Server hosted runtime config/composition/module/tests
- narrow startup/state/docs/manifest changes directly required
- Server control report

protected_scope:
- accepted Worktree and Storage source snapshots
- migrations and schema
- Core, API, CLI and Deployment product files
- sibling control files and workflows

forbidden_early_work:
- API-P8 public DTO/status contract
- SRV-P7B5 readiness/status mapping or routes
- CLI or Deployment behavior
- detached/multiple runtime tasks, nested runtime, block_on or internal HTTP
- unbounded retry, provider behavior, hard delete or automatic repair

completion_requirements:
- component/server advances with a real code-bearing hosted runtime commit
- committed crates/haze-sync-server/control/report.md exists
- report phase SRV-P7B4-HOSTED-WORKTREE-RUNTIME
- report chat name server — W1 SRV-P7B4 Hosted Worktree Runtime
- protected owner snapshots remain unchanged
- exact final code-bearing SHA is recorded
- authoritative DB-capable Component CI is green on exact final code-bearing SHA, or report records an honest blocker/failure with exact evidence

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> mandatory SRV-P7B4 clean-code review
- red CI -> rotate fixer from exact diagnostics artifact evidence
- owner contract defect -> route to exact Worktree or Storage owner
- API-P8 and SRV-P7B5 remain blocked until SRV-P7B4 CLEAN_ACCEPT
