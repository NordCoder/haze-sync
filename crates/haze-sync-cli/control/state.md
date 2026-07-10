# Control State

component: cli
branch: component/cli
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: none

wave: W1
phase: CLI-P6-BLOCKED

implementation_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
ci_workflow: Component CI
ci_run_id: 29084418994
ci_run_number: 1047
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT
blocker: GDA-P7C is accepted, but WT-P8C and Server/API bootstrap/sync operator contracts and runtime fan-in are unavailable
unblock_condition: green WT-P8C acceptance plus accepted Server/API operator and runtime fan-in contracts, or a dedicated CLI fan-in contract
