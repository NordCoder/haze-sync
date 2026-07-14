# Control State

component: api
branch: component/api
status: PROMPT_READY

active_slot_override: PREVIOUS_DEPENDENCY_HOLD_RESOLVED
previous_blocked_phase: API-P8-BLOCKED-BY-SRV-P7B4
previous_blocked_status: superseded
worker_must_execute_current_prompt: yes

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: api — W1 API-P8 Main Sync
prompt_revision: refreshed after stale worker read; SRV-P7B5 dependency is CLEAN_ACCEPT and no dependency blocker remains for API-P8-PRE-SYNC

wave: W1
phase: API-P8-PRE-SYNC

implementation_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT_FOR_API_P7C
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

resolved_dependencies:
- srv_p7b3: CLEAN_ACCEPT
- srv_p7b4: CLEAN_ACCEPT
- srv_p7b5: CLEAN_ACCEPT
- accepted_server_sha: 1d1fc8ca62c97db041cca09dd8316370285dfba1
- server_clean_report_commit: 23b49dff38c8b6997193b6224681feada3e09c1e
- server_clean_report_blob: 2416d7280883761bf90117a7e3dbfc41147756b9
- dependency_blocker_remaining: no

accepted_api_baseline:
- code_bearing_sha: 3109c0fd9b456ca5fd8db099cd83843dae44cef9
- prior_ci_run_id: 29093081652
- prior_ci_run_number: 1217
- prior_ci_conclusion: success

branch_sync:
- original_pre_sync_head: c07c3d76990c12cac468bfda8c5dcb3637d97485
- target_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- relation_at_assignment: diverged
- behind_main_by_at_assignment: 152
- api_local_commits_at_assignment: 12
- method_required: normal merge commit; no rebase/history rewrite

slot_scope:
- synchronize component/api with exact current main
- preserve API-local history
- no API-P8 routes/DTO/manual product work yet
- exact post-sync Component CI required

next_gate:
- SELF_ACCEPT sync plus green exact-SHA CI -> API-P8 implementation prompt
- substantive conflict or red CI -> focused sync fixer
- CLI-P6A and Deployment remain blocked until API-P8 acceptance
