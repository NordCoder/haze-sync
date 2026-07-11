# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: worktree — W1 WT-P9C CI Fix
prompt_revision: verified by Orchestrator after WT-P9C clean review reported tooling block

wave: W1
phase: FIX-WT-P9C-CI

implementation_status: SELF_ACCEPT
clean_review_status: CLEAN_BLOCKED_BY_TOOLING
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: b9520336c97ccf0e1b1905c7c9ad1d3a4a2d061e
ci_run_id: 29148826285
ci_run_number: 1664
ci_run_attempt: 1
ci_artifact_name: ci-diag__component-worktree__wf-component-ci__run-29148826285__attempt-1
ci_artifact_id: 8247562500
ci_artifact_expires_at: 2026-07-12T10:08:37Z
known_failed_checks:
- Finalize CI diagnostics
architect_status: ARCHITECT_ACCEPT
next_gate_after_fix: close WT-P9C as CLEAN_ACCEPT only after a verified green post-fix code-bearing CI run; then move Worktree to component-complete fan-in hold and unblock explicit Server-owned fan-in planning
fan_in_note: Server remains blocked; concrete hosting and repair execution are not part of this fixer
