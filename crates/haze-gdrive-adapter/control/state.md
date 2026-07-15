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
assigned_chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P3 CI

wave: W1
phase: FIX-GDA-GDA-P3-CI
implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_FINALIZER_ONLY
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

failing_candidate:
- code_bearing_sha: 6044c6d2cb160250a890415c0ff0d2785be6a981
- implementation_report_blob: 5f0aea2859eba8575f3c8e302b72a09ca7aa24fc
- ci_run_id: 29446145224
- ci_run_number: 2022
- ci_attempt: 1
- artifact_id: 8355565313
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29446145224__attempt-1

protected_scope:
- artifact-proven fix only
- preserve OAuth/credential/auth boundaries and redaction
- no next GDrive product phase
- no Server/API/Storage/scheduler/Deployment/sibling/workflow changes

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> focused OAuth/security clean review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
- unresolved contract issue -> FIX_BLOCKED_BY_CONTRACT
