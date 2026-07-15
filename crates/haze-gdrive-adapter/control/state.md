# Control State

component: gdrive-adapter
repository: NordCoder/haze-sync
branch: component/gdrive-adapter
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/gdrive-adapter
default_branch_control_is_active: no

active_prompt: crates/haze-gdrive-adapter/control/prompt.md
active_report: crates/haze-gdrive-adapter/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P1 CI

wave: W1
phase: FIX-GDA-GDA-P1-CI
implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_FINALIZER_ONLY
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

failing_candidate:
- code_bearing_sha: afd263d621723952f11ecd0c16c09e209710feac
- implementation_report_blob: 7090b1b71ebca300847f1a2310ae4dd761c00a52
- ci_run_id: 29431493842
- artifact_id: 8349581571
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29431493842__attempt-1

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> focused GDrive clean review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
