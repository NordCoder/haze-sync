# Control State

component: worktree
branch: component/worktree
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: none

wave: W1
phase: WT-COMPLETE-FAN-IN

implementation_status: SELF_ACCEPT
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
ci_workflow: Component CI
ci_code_bearing_sha: 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
ci_run_id: 29152965199
ci_run_number: 1665
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT
blocker: component-local Worktree plan is complete; accepted product snapshot requires explicit cross-component fan-in before Server hosting
unblock_condition: explicit Orchestrator fan-in/integration prompt pinned to Worktree SHA 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
server_branch_note: component/server currently contains the placeholder Worktree crate and must not claim SRV-P7 completion without snapshot fan-in
