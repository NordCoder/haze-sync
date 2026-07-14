# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 API-P8 Worktree HTTP Fan-In
prompt_revision: verified by Orchestrator after API-P8 CLEAN_ACCEPT; exact API blobs and Server HTTP semantics pinned; formatting/style excluded from blocking scope

wave: W1
phase: SRV-API-P8-WORKTREE-HTTP-FAN-IN

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: NOT_RUN
known_failed_checks: []

accepted_server_baseline:
- code_bearing_sha: 1d1fc8ca62c97db041cca09dd8316370285dfba1
- clean_report_commit: 23b49dff38c8b6997193b6224681feada3e09c1e
- clean_report_blob: 2416d7280883761bf90117a7e3dbfc41147756b9
- ci_run_id: 29289080020
- ci_run_number: 1912
- ci_conclusion: success
- db_capable: yes

accepted_api_p8:
- code_bearing_sha: 56ae94570441d68715f34b5d54381a0fc4d7c231
- clean_report_commit: abbfb0c09f62d9f778a90207609318bda85de9ba
- clean_report_blob: 401dc1c2fe9d5ce91a1aa248be8ed7d27a8275d7
- ci_run_id: 29315949762
- ci_run_number: 1925
- ci_conclusion: success

accepted_api_blobs:
- dto_worktree: 7c592184d58c1cda98314fd0e2eae8baed1de1e8
- dto_mod: 5534f79606639fb13857de0729793d9083523e03
- routes_worktree: 032f43a1a513e7fb25d6103de281f7e5d8e08930
- routes_mod: 439bebd09d3aa9252188d2d3eb106e65943c5846
- fixture: da0a197b1e6d5425f05cfd6fe772a4c96f6a830c
- fixture_test: 968f1e9825a2c54385615bf75db54a975218fd20
- contract_doc: f2c8d2d53c09a0e1ac0caf3ac8c6ee8d15754009

implementation_goal:
- exact API-P8 product fan-in
- cloneable Server HTTP control handle separated from unique host shutdown ownership
- GET /v1/admin/worktree/status
- POST /v1/admin/worktree/sync-once
- Admin auth, strict empty request, submission-only response
- no completion wait/poll/task/retry
- DB-capable exact-SHA Component CI

http_contract:
- status success: 200
- sync accepted: 202
- sync busy: 409
- sync not_started/cancelling/shutdown/unavailable: 503
- sync failed: 500
- auth missing/invalid: 401
- auth non_admin: 403

protected_scope:
- no API semantic edits
- no Worktree/Storage/Core/CLI/Deployment product changes
- no migrations/workflows
- no task/poller/retry/completion watcher
- no raw internal or secret-bearing output
- no unrelated cleanup

next_gate:
- SELF_ACCEPT plus green DB-capable exact-SHA CI -> focused Server functional review
- CLEAN_ACCEPT after review -> CLI-P6A control-slot resolution
- Deployment remains blocked
