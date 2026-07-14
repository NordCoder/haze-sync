# Control State

component: cli
branch: component/cli
status: PROMPT_READY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: cli — W1 CLI-P6A Main Sync
prompt_revision: previous API/Server dependency hold resolved; executable pre-sync slot pinned to exact main and accepted Server/API SHAs

wave: W1
phase: CLI-P6A-PRE-SYNC

implementation_status: NOT_STARTED_SYNC
clean_review_status: CLEAN_ACCEPT_EXISTING_CLI_P5
ci_status: NOT_RUN_POST_SYNC
architect_status: ARCHITECT_ACCEPT_EXISTING_BOUNDARY
known_failed_checks: []

branch_state:
- pre_sync_head: d923ad4d66416344ea166097fe1e53728701652b
- exact_main_sha: c1e69a664388b0cba028170e8398b9088218957d
- merge_base: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
- behind_main: 139
- cli_local_commits: 12
- normal_merge_required: yes
- rebase_or_history_rewrite_forbidden: yes

resolved_dependencies:
- API-P8 CLEAN_ACCEPT at 56ae94570441d68715f34b5d54381a0fc4d7c231
- Server Worktree HTTP CLEAN_ACCEPT at 50461354c18ddc4d2e47202d9303b4358a27ee45
- Server clean report blob e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5
- Server DB-capable CI run 29326558901 success

protected_scope:
- synchronization only
- no CLI-P6A status/sync-once product work
- no sibling branch changes
- no PR merge or draft-state change

next_gate:
- SELF_ACCEPT plus green exact post-sync CI -> CLI-P6A product implementation slot
- synchronization defect -> focused fixer
- Deployment remains blocked until CLI-P6A acceptance
