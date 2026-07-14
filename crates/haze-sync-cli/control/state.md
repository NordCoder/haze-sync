# Control State

component: cli
branch: component/cli
status: PROMPT_READY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: cli — W1 CLI-P6A CI Diagnostics Fix
prompt_revision: verified by Orchestrator after SELF_NEEDS_FIX; artifact-first diagnosis required

wave: W1
phase: FIX-CLI-P6A-CI

implementation_status: SELF_NEEDS_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_DIAGNOSTICS_AVAILABLE
architect_status: ARCHITECT_ACCEPT_EXISTING_BOUNDARY
known_failed_checks: [diagnostics finalizer; exact check pending artifact read]

candidate:
- code_bearing_sha: 6c4c2ba4a66999e02542083512587d0d6ad8d437
- implementation_report_blob: 5df1ebcc3696b1d61d514e005b2dbaebe4213622
- ci_run_id: 29331452103
- ci_run_number: 1946
- ci_run_attempt: 1
- ci_conclusion: failure

artifact:
- id: 8310157600
- name: ci-diag__component-cli__wf-component-ci__run-29331452103__attempt-1
- digest: sha256:49a5d411691f424fcd8aec8893086846674f4707e580f58f0302c10f1d4770d3
- expires_at: 2026-07-15T12:11:04Z
- summary_manifest_logs_required: yes

protected_scope:
- minimum artifact-proven CLI correction only
- accepted API-P8 blobs must remain exact
- no Server/Worktree/Storage/Core/GDrive/Deployment changes
- no workflow weakening or diagnostic suppression
- no unrelated cleanup

next_gate:
- FIX_COMPLETE plus green exact-SHA CI -> focused CLI-P6A functional review
- unresolved evidence -> honest blocked status
- Deployment remains blocked until CLI-P6A CLEAN_ACCEPT
