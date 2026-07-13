# Control State

component: api
branch: component/api
status: PROMPT_READY

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: api — W1 API-P8 Main Sync
prompt_revision: verified by Orchestrator after SRV-P7B5 CLEAN_ACCEPT; explicit main synchronization required before API-P8 product work

wave: W1
phase: API-P8-PRE-SYNC

implementation_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT_FOR_API_P7C
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

accepted_api_baseline:
- code_bearing_sha: 3109c0fd9b456ca5fd8db099cd83843dae44cef9
- prior_ci_run_id: 29093081652
- prior_ci_run_number: 1217
- prior_ci_conclusion: success

branch_sync:
- pre_sync_head: c07c3d76990c12cac468bfda8c5dcb3637d97485
- target_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- relation: diverged
- behind_main_by: 152
- api_local_commits: 12
- method_required: normal merge commit; no rebase/history rewrite

accepted_server_contract:
- srv_p7b5_code_bearing_sha: 1d1fc8ca62c97db041cca09dd8316370285dfba1
- clean_report_commit: 23b49dff38c8b6997193b6224681feada3e09c1e
- clean_report_blob: 2416d7280883761bf90117a7e3dbfc41147756b9
- clean_status: CLEAN_ACCEPT

slot_scope:
- synchronize component/api with exact current main
- preserve API-local history
- no API-P8 routes/DTO/manual product work yet
- exact post-sync Component CI required

next_gate:
- SELF_ACCEPT sync plus green exact-SHA CI -> API-P8 implementation prompt
- substantive conflict or red CI -> focused sync fixer
- CLI-P6A and Deployment remain blocked until API-P8 acceptance
