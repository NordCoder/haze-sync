# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: worktree — W1 WT-P11 CI Fix
prompt_revision: verified by Orchestrator after exact-SHA WT-P11 diagnostics-finalizer failure and artifact publication

wave: W1
phase: WT-P11-CI-FIX

implementation_status: BLOCKED_BY_TOOLING
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED_OWNER_BOUNDARY_ALREADY_DECIDED
ci_status: CI_RED_DIAGNOSTICS_AVAILABLE
known_failed_checks:
- diagnostics finalizer

failed_candidate:
- code_bearing_sha: 8a21497c845710a4205cb91b2af7d13b5346bd95
- implementation_report_commit: 0873d1ea23627f8f7ac62ee3437fbc3ad92600bf
- implementation_report_blob: f8ab6904d5a6b4555fc9e0f39ed30eacefa69bce
- post_candidate_changes: report-only

failed_ci_evidence:
- workflow: Component CI
- run_id: 29265781943
- run_number: 1865
- run_attempt: 1
- head_sha: 8a21497c845710a4205cb91b2af7d13b5346bd95
- conclusion: failure
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: failure
- diagnostics_upload: success

artifact_evidence:
- artifact_id: 8285369808
- artifact_name: ci-diag__component-worktree__wf-component-ci__run-29265781943__attempt-1
- artifact_digest: sha256:8703d14197a2a867bd633f9f25d8168f99d3b50a209fdd3d130c4ad6165a0c50
- artifact_expired: false
- artifact_expires_at: 2026-07-14T16:19:06Z

archived_completed_slot:
- prompt_index: crates/haze-sync-worktree/control/log/20260713-162100Z-W1-WT-P11-HOSTED-RUNTIME-CONTRACT-implementation-worker-prompt.md
- report_index: crates/haze-sync-worktree/control/log/20260713-162100Z-W1-WT-P11-HOSTED-RUNTIME-CONTRACT-implementation-worker-report.md
- prompt_blob: 1ba6be8e41f5ea03a7cf096a0c60ac927d526108
- report_blob: f8ab6904d5a6b4555fc9e0f39ed30eacefa69bce
- report_commit: 0873d1ea23627f8f7ac62ee3437fbc3ad92600bf

completion_requirements:
- exact artifact read and root cause recorded
- only artifact-proven Worktree scope changed
- final code-bearing fix SHA recorded
- exact-SHA Component CI green or honest blocker
- committed FIX report exists

next_gate_after_fix:
- FIX_COMPLETE plus green exact-SHA CI -> mandatory WT-P11 clean-code review
- red CI -> continue fixer routing from new diagnostics artifact
- Server SRV-P7B4 remains blocked until WT-P11 CLEAN_ACCEPT and exact-SHA fan-in
