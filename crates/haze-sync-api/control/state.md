# Control State

component: api
repository: NordCoder/haze-sync
branch: component/api
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/api
default_branch_control_is_active: no

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: api — W1 FIX-API-GDA-P1 CI

wave: W1
phase: FIX-API-GDA-P1-CI
implementation_status: COMPLETE_PENDING_CI_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_FINALIZER_ONLY
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

failing_candidate:
- code_bearing_sha: 8354b7d0b9bb9152e5609d36814222741b70d14e
- implementation_report_blob: 490921fb423bb0c6119bb966cab49f72d6f0e619
- ci_run_id: 29437624209
- ci_run_number: 2009
- ci_attempt: 2
- artifact_id: 8352138123
- artifact_name: ci-diag__component-api__wf-component-ci__run-29437624209__attempt-2

protected_scope:
- fix only artifact-proven failure
- preserve passive GDrive state and compare-and-commit contracts
- no Server, Storage, Core, provider, scheduler, sibling or workflow changes
- no new API product phase

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> focused API clean review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
- contract issue -> FIX_BLOCKED_BY_CONTRACT
