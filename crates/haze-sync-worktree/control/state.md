# Control State

component: worktree
branch: component/worktree
status: ACCEPTED_HOLD

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: none
assigned_chat_name: none
prompt_revision: verified by Orchestrator during full component dependency audit

wave: W1
phase: WT-P10-ACCEPTED-HOLD

implementation_status: SELF_ACCEPT
fix_status: FIX_COMPLETE
clean_review_status: CLEAN_ACCEPT
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_GREEN
accepted_code_bearing_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
ci_run_id: 29167289593
ci_run_number: 1799
ci_run_attempt: 1
known_failed_checks: []

accepted_contract:
- awaitable boxed Send WorktreeRuntimeCycle future
- async poll awaits at most one cycle
- cooperative cancellation and no-overlap guarantees
- explicit non-automatic mutation-free DryRun
- full-scan and watcher-fallback correctness
- bounded import/delete/export budgets and summary validation
- safe count/category-only status and Debug output
- synchronous filesystem/scanner/planner/materializer/echo/trash/doctor contracts unchanged

owner_snapshot_status:
- component/worktree is ahead of accepted SHA by control-only commits
- no later product or tooling commit invalidates accepted snapshot

parallel_owner_status:
- STOR-P10 CLEAN_ACCEPT at 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- SRV-P7B2 CLEAN_ACCEPT at 647dce7b624d67663632808906896cb6745ea7e7

integration_status:
- component/server does not yet contain the exact accepted Worktree product snapshot
- explicit Server exact-SHA fan-in is required before SRV-P7B3

blocked_downstream:
- SRV-P7B3 waits for accepted Worktree and Storage product fan-in, green DB-capable CI and integration clean review

unblock_condition:
- explicit Worktree-owned integration defect or later Worktree contract phase only