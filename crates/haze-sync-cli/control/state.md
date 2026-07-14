# Control State

component: cli
branch: component/cli
status: PROMPT_READY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: cli — W1 CLI-P6A Worktree Operator Commands
prompt_revision: verified by Orchestrator after valid normal main sync and green exact-SHA CI; accepted API/Server contracts pinned

wave: W1
phase: CLI-P6A-WORKTREE-OPERATOR-COMMANDS

implementation_status: NOT_STARTED_PRODUCT
clean_review_status: NOT_STARTED
ci_status: NOT_RUN_PRODUCT
architect_status: ARCHITECT_ACCEPT_EXISTING_BOUNDARY
known_failed_checks: []

synchronized_baseline:
- post_sync_sha: 8a3012a20440066422e7ad6c4e52d1a859b1bd51
- exact_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- pre_sync_ci_run_id: 29329332012
- pre_sync_ci_run_number: 1941
- pre_sync_ci_conclusion: success

accepted_api_p8:
- code_bearing_sha: 56ae94570441d68715f34b5d54381a0fc4d7c231
- clean_report_blob: 401dc1c2fe9d5ce91a1aa248be8ed7d27a8275d7
- exact_fan_in_required: yes

accepted_server_http:
- code_bearing_sha: 50461354c18ddc4d2e47202d9303b4358a27ee45
- clean_report_blob: e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5
- ci_run_id: 29326558901
- ci_run_number: 1940
- db_capable: yes

implementation_goal:
- haze-sync worktree status
- haze-sync worktree sync-once
- exact accepted method/path/request/response vocabulary
- deterministic client boundary and rendering
- no concrete fake-success transport
- no polling/retry/wait/ticket behavior
- no local runtime, DB, filesystem or provider work

protected_scope:
- exact API-P8 product fan-in only outside CLI
- no API semantic edits
- no Server/Worktree/Storage/Core/GDrive/Deployment product changes
- no migrations/workflows
- no token CLI flags or secret output
- no unrelated cleanup

next_gate:
- SELF_ACCEPT plus green exact-SHA CI -> focused CLI-P6A functional review
- substantive defect -> focused CLI fixer
- Deployment remains blocked until CLI-P6A CLEAN_ACCEPT
