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
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-GDA-P1 State Routes and Transactions

wave: W1
phase: SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

accepted_baseline:
- server_code_bearing_sha: 50461354c18ddc4d2e47202d9303b4358a27ee45
- server_clean_report_blob: e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5

accepted_dependencies:
- api_gdrive_sha: c60c3976696da1970d539e5cff6e9f74a61fc10e
- api_clean_report_blob: 55ff6047c9c6c0f6f548f10197b76706c0a244e1
- storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- storage_clean_report_blob: 4584b8705221d3cd2aa43b5776674b3a1ec9a0f4
- architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4

required_surface:
- authenticated private/admin GDrive state reads
- compare-and-commit route and application execution
- caller-owned PostgreSQL transactions
- exact Storage invariant and outcome mapping
- rollback, replay, concurrency, isolation and redaction tests
- DB-capable exact-SHA CI

protected_scope:
- one active Server phase only
- no API/Storage contract changes
- no provider/OAuth/Core policy/status-control/CLI/Deployment work
- no sibling or workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA DB-capable green CI -> focused Server clean/functional review
- implementation or CI defect -> fixer loop
- owner contract mismatch -> BLOCKED_BY_CONTRACT
