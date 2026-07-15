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
active_agent_role: implementation-worker
assigned_chat_name: api — W1 API-GDA-P1 GDrive Contracts

wave: W1
phase: API-GDA-P1-CONTRACTS
implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_GDRIVE_FAN_IN

accepted_inputs:
- api_baseline_sha: 56ae94570441d68715f34b5d54381a0fc4d7c231
- storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- storage_clean_report_blob: 4584b8705221d3cd2aa43b5776674b3a1ec9a0f4
- architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4
- exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d

required_surface:
- GET adapter GDrive state contract
- POST compare-and-commit contract
- bounded typed DTOs and route helpers
- stable safe error vocabulary
- private raw-cursor redaction boundary
- deterministic fixtures and tests

protected_scope:
- passive API only
- no Server registration or runtime behavior
- no Storage/SQLx/Core/provider/OAuth/scheduler work
- no status ingestion/operator controls
- no sibling component or workflow changes

next_gate:
- SELF_ACCEPT plus exact-SHA green CI -> focused API clean review
- missing accepted contract -> BLOCKED_BY_CONTRACT
- implementation or CI defect -> fixer loop
