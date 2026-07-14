# Control State

component: api
branch: component/api
status: PROMPT_READY

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: api — W1 API-P8 Worktree Status Contract
prompt_revision: verified by Orchestrator after successful exact-main synchronization and SRV-P7B5 CLEAN_ACCEPT; formatting/style excluded from blocking scope

wave: W1
phase: API-P8-WORKTREE-STATUS-CONTRACT

implementation_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT_EXISTING_PASSIVE_BOUNDARY
known_failed_checks: []

accepted_api_sync:
- merge_sha: d0e8ef0705b7c0456f2cb1359428ff30b90961b4
- exact_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- sync_report_commit: 0a3efb21ed256f49b0ad92b445b55bbf7fdb28a0
- sync_report_blob: 8de46f68c64532aad6fc983d4231f73563dcd9d5
- ci_run_id: 29312597987
- ci_run_number: 1913
- ci_conclusion: success

accepted_server_contract:
- srv_p7b5_code_bearing_sha: 1d1fc8ca62c97db041cca09dd8316370285dfba1
- clean_report_commit: 23b49dff38c8b6997193b6224681feada3e09c1e
- clean_report_blob: 2416d7280883761bf90117a7e3dbfc41147756b9
- clean_status: CLEAN_ACCEPT

implementation_goal:
- passive secret-safe Worktree status DTO matching accepted Server vocabulary
- passive admin-only sync-once submission contract
- submission outcomes only; no completion/ticket/runtime behavior
- stable JSON vocabulary and compatibility fixtures
- no Server dependency or route registration

protected_scope:
- no Axum/router/runtime wiring
- no Server/Worktree/Storage/Core/CLI/Deployment product changes
- no DB/object-store/provider/filesystem work
- no task/poller/retry/wait loop
- no raw paths/errors/payloads/tokens/cursors/idempotency/tickets
- no migrations/workflows/unrelated cleanup

completion_requirements:
- real API code-bearing commit
- exact final SHA recorded
- authoritative exact-SHA Component CI green or honest blocker
- committed IMPLEMENTATION report exists

next_gate:
- SELF_ACCEPT plus green exact-SHA CI -> focused API-P8 functional review
- substantive defect -> focused fixer
- CLEAN_ACCEPT after review -> CLI-P6A may begin and Server HTTP fan-in can be scheduled
- Deployment remains blocked until downstream acceptance
