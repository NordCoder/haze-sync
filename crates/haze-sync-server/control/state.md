# Control State

component: server
repository: NordCoder/haze-sync
branch: component/server
status: BLOCKED_BY_DEPENDENCY
repository_access_verified: yes
control_ref_source: component/server
default_branch_control_is_active: no

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: none
active_agent_role: orchestrator-hold
assigned_chat_name: none

wave: W1
phase: SRV-GDRIVE-FAN-IN-BLOCKED-HOLD
implementation_status: BASELINE_ACCEPTED
fix_status: FIX_COMPLETE
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN_DB_VERIFIED
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

accepted_baseline:
- final_code_bearing_sha: 50461354c18ddc4d2e47202d9303b4358a27ee45
- clean_report_blob: e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5
- ci_run_id: 29326558901
- ci_run_number: 1940
- db_capable: yes

accepted_dependencies:
- storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- storage_clean_report_blob: 4584b8705221d3cd2aa43b5776674b3a1ec9a0f4

blocking_dependency:
- API-GDA-P1 must receive CLEAN_ACCEPT
- Orchestrator must provide exact accepted API SHA and clean-review blob

next_gate:
- API-GDA-P1 CLEAN_ACCEPT -> Server GDrive application/transaction PROMPT_READY
- no Server product work before that gate
