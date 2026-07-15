# Control State

component: storage
repository: NordCoder/haze-sync
branch: component/storage
status: PROMPT_READY
repository_access_verified: yes
migrated_repository_lookup_required: no
control_ref_source: component/storage
default_branch_control_is_active: no

active_prompt: crates/haze-sync-storage/control/prompt.md
active_report: crates/haze-sync-storage/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: storage — W1 FIX-STOR-GDA-P1 Rust Workspace CI

wave: W1
phase: FIX-STOR-GDA-P1-RUST-WORKSPACE-CI
implementation_status: SELF_NEEDS_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN
ci_status: CI_RED
known_failed_checks:
- Rust workspace job 87373222668

failing_candidate:
- code_bearing_sha: 4f539e32ae8768aeeb9cda11f388745c3771496c
- implementation_report_blob: b3a0113b6db03e8ac410e9ca0708a38ae4f22d5c
- ci_run_id: 29421594032
- ci_run_number: 1987
- ci_run_attempt: 1
- postgres_verification: success
- rust_workspace: failure

diagnostics:
- artifact_id: 8345471214
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29421594032__attempt-1
- primary_log_source: artifact
- raw_job_logs_fallback_only: yes

protected_scope:
- fix only artifact-proven Rust workspace failure
- preserve migration 0011 and durable-state contracts
- no sibling component or workflow changes
- no new Storage product phase

next_gate:
- FIX_COMPLETE plus full exact-SHA CI green -> Storage clean/DB review
- artifact unavailable -> FIX_BLOCKED_BY_LOGS
- contract issue -> FIX_BLOCKED_BY_CONTRACT
