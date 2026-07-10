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
blocker: WT-P8 and GDA-P7C are implemented but formally red and not fan-in accepted; Server/API operator contracts are unavailable
unblock_condition: green WT-P8 and GDA-P7C acceptance plus Server/API bootstrap and sync contracts, or a dedicated CLI fan-in contract
