# Control State

component: storage
branch: component/storage
status: PROMPT_READY

active_prompt: crates/haze-sync-storage/control/prompt.md
active_report: crates/haze-sync-storage/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: storage — W1 STOR-P10 CI Fix
prompt_revision: verified by Orchestrator after STOR-P10 completed implementation and exact diagnostics artifact metadata was retrieved

wave: W1
phase: FIX-STOR-P10-CI

implementation_status: BLOCKED_BY_TOOLING
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: 63d80764933cba5f23fb43bad44201a75e1dc16a
ci_run_id: 29166287661
ci_run_number: 1765
ci_run_attempt: 1
known_failed_checks:
- Finalize CI diagnostics

artifact_name: ci-diag__component-storage__wf-component-ci__run-29166287661__attempt-1
artifact_id: 8252246409
artifact_head_sha: 63d80764933cba5f23fb43bad44201a75e1dc16a
artifact_digest: sha256:c2e86e52d1bd342fd577503042c4e4640591179a08fae678de1a27d22d4e0f2c
artifact_size_bytes: 13317
artifact_expires_at: 2026-07-12T20:03:15Z
artifact_status: AVAILABLE_UNEXPIRED

accepted_implementation:
- versioned Worktree instance/root-fingerprint binding
- per-adapter present/tombstoned durable path state
- fail-safe legacy migration strategy
- caller-transaction-owned path state and exact contiguous cursor repositories
- safe redacted errors and deterministic schema/test-support coverage

separate_tooling_blocker:
- mandatory strict PostgreSQL migration/state/cursor tests were not executed
- ordinary cargo test success with ignored DB tests is not acceptance evidence
- no clean-code review until dedicated DB-capable verification succeeds

parallel_owner_status:
- WT-P10 is independently in tooling correction
- SRV-P7B2 is independently in tooling correction

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and exact accepted SHAs are synchronized
- Deployment migration work remains blocked

next_gate_after_fix:
- artifact correction and independently verified green post-fix code-bearing CI -> dedicated STOR-P10 PostgreSQL verification prompt
- only after required DB/migration/repository/cursor evidence succeeds -> mandatory STOR-P10 clean-code review
- failed or blocked fix -> route exact artifact/scope/contract/tooling decision
