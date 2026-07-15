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
active_agent_role: clean-code-reviewer
assigned_chat_name: api — W1 API-GDA-P1 Clean Review

wave: W1
phase: API-GDA-P1-CLEAN-REVIEW
implementation_status: COMPLETE_AFTER_FIX
fix_status: FIX_COMPLETE
clean_review_status: NOT_STARTED
ci_status: CI_GREEN
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN
known_failed_checks: []

review_candidate:
- code_bearing_sha: 77945118a37c6e8efec04ecd054e0e5a5e4435ba
- implementation_report_blob: 490921fb423bb0c6119bb966cab49f72d6f0e619
- fixer_report_blob: 5b91145a8c0e8746476e3bd5b058e2fcb437f517
- ci_run_id: 29440533856
- ci_run_number: 2017
- ci_conclusion: success
- storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- storage_clean_report_blob: 4584b8705221d3cd2aa43b5776674b3a1ec9a0f4

protected_scope:
- review passive API contracts only
- no product, Server, Storage, provider, status-control, sibling or workflow changes

next_gate:
- CLEAN_ACCEPT -> Server GDrive application/transaction phase
- CLEAN_NEEDS_FIX -> focused API fixer loop
