# Control State

component: worktree
branch: component/worktree
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: none
assigned_chat_name: none
prompt_revision: verified by Orchestrator after WT-P10 CLEAN_ACCEPT

wave: W1
phase: WT-P10-COMPLETE-FAN-IN

implementation_status: SELF_ACCEPT
fix_status: FIX_COMPLETE
clean_review_status: CLEAN_ACCEPT
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_GREEN
ci_workflow: Component CI
ci_code_bearing_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
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

parallel_owner_status:
- STOR-P10 requires branch synchronization and executable PostgreSQL verification
- SRV-P7B2 requires branch synchronization and executable PostgreSQL-capable CI

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and exact accepted SHAs are synchronized

unblock_condition:
- explicit Orchestrator fan-in/integration prompt pinned to accepted SHA 1942946331e8362f19907ab6ad4eb779da70fd57