# Control State

component: obsidian-plugin
repository: NordCoder/haze-sync
branch: component/obsidian-plugin
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/obsidian-plugin
default_branch_control_is_active: no

active_prompt: apps/haze-obsidian-plugin/control/prompt.md
active_report: apps/haze-obsidian-plugin/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: obsidian-plugin — W1 FIX-OBS-FAN-IN-P1 CI

wave: W1
phase: FIX-OBS-FAN-IN-P1-CI
implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED_FINALIZER_ONLY
architect_status: ARCHITECT_ACCEPT_COMPONENT_COMPLETE

failing_candidate:
- code_bearing_sha: 517b8cd77dc7c24eec78ae6c2f1ab55e7a8aeb3a
- implementation_report_blob: ffdc199c2e92e5f5a3be8bdb6666f5a9bd8ee5e3
- ci_run_id: 29434046014
- ci_run_number: 1999
- node_job_id: 87416059909
- rust_workspace_job_id: 87416059992
- artifact_id: 8350606040
- artifact_name: ci-diag__component-obsidian-plugin__wf-component-ci__run-29434046014__attempt-1

next_gate:
- FIX_COMPLETE plus full exact-SHA green CI -> focused Obsidian clean/integration review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
