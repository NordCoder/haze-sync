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
active_agent_role: clean-code-reviewer
assigned_chat_name: server — W1 SRV-GDA-P1 Clean Functional Review Rerun

wave: W1
phase: SRV-GDA-P1-CLEAN-FUNCTIONAL-REVIEW-RERUN
implementation_status: COMPLETE_AFTER_CONTINUATION
fix_status: FIX_COMPLETE
clean_review_status: RERUN_NOT_STARTED
ci_status: CI_GREEN_DB_VERIFIED
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

review_candidate:
- code_bearing_sha: c023b83e1e6f502e7d2261acccb871dd5588edf1
- prior_clean_review_report_blob: d4fe8c9150184f34383d708048d402df7c008078
- outcome_mapping_fixer_report_blob: 2c8feb9d05e07c68f9e6b501a4098d96d2f0f1c0
- ci_run_id: 29494321838
- ci_run_number: 2045
- ci_conclusion: success
- db_capable: yes

accepted_dependencies:
- api_gdrive_sha: c60c3976696da1970d539e5cff6e9f74a61fc10e
- storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84

required_review:
- invalid_cursor_state classification for persisted generation mismatch
- internal envelope for internal/unexpected failures
- validation_failed limited to genuine validation
- PostgreSQL rollback and secrecy evidence
- preserved route, transaction, concurrency, isolation and owner identity boundaries

next_gate:
- CLEAN_ACCEPT -> GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT
- CLEAN_NEEDS_FIX -> focused Server fixer loop
