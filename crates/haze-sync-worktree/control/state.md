# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: worktree — W1 WT-P10 CI Fix
prompt_revision: verified by Orchestrator after WT-P10 completed implementation and exact diagnostics artifact metadata was retrieved

wave: W1
phase: FIX-WT-P10-CI

implementation_status: BLOCKED_BY_TOOLING
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: b3c62d4e1655d0a595290a87c593974a5ed1a3dc
ci_run_id: 29165931603
ci_run_number: 1737
ci_run_attempt: 1
known_failed_checks:
- Finalize CI diagnostics

artifact_name: ci-diag__component-worktree__wf-component-ci__run-29165931603__attempt-1
artifact_id: 8252153523
artifact_head_sha: b3c62d4e1655d0a595290a87c593974a5ed1a3dc
artifact_digest: sha256:e69d46bb8077a0ec7496df3480e270d95761191d5b7e612c9d06c45275924fc9
artifact_size_bytes: 2977
artifact_expires_at: 2026-07-12T19:51:15Z
artifact_status: AVAILABLE_UNEXPIRED

accepted_implementation:
- awaitable boxed Send Worktree cycle contract
- async poll awaits at most one cycle
- cooperative cancellation and no-overlap guarantees
- explicit inert planning-only DryRun semantics
- full-scan, watcher fallback, budgets and summary validation preserved
- no Server, Storage, blocking bridge or hidden task

parallel_owner_status:
- STOR-P10 is independently in tooling correction
- SRV-P7B2 is independently in tooling correction

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and exact accepted SHAs are synchronized

next_gate_after_fix:
- FIX_COMPLETE plus independently verified green post-fix code-bearing Component CI -> mandatory WT-P10 clean-code review
- failed or blocked fix -> route exact artifact/scope/contract/tooling decision; no clean review
