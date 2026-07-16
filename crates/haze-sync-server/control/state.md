# Control State

component: server
repository: NordCoder/haze-sync
branch: component/server
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/server
default_branch_control_is_active: no

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 FIX-SRV-GDA-P1 CI

wave: W1
phase: FIX-SRV-GDA-P1-CI
implementation_status: COMPLETE_PENDING_CI_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_FINALIZER_ONLY
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

failing_candidate:
- code_bearing_sha: 94375e36976c87b62e524c0f1cb490224b778179
- implementation_report_blob: efc05e8e2a903e7197b90cc996d471f753f5a7fa
- ci_run_id: 29488329906
- ci_run_number: 2035
- ci_attempt: 1
- artifact_id: 8371364050
- artifact_name: ci-diag__component-server__wf-component-ci__run-29488329906__attempt-1

protected_scope:
- artifact-proven fix only
- preserve completed route, transaction and PostgreSQL verification surface
- no owner-contract or sibling/workflow changes

next_gate:
- FIX_COMPLETE plus full exact-SHA DB-capable green CI -> focused Server clean review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
