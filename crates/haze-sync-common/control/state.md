# Control State

component: common
branch: component/common
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-common/control/prompt.md
active_report: crates/haze-sync-common/control/report.md
active_agent_role: none

wave: W1
phase: CMM-COMPLETE-FAN-IN

implementation_status: SELF_ACCEPT
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
ci_workflow: Component CI
ci_run_id: 29086420405
ci_run_number: 1095
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT
blocker: component-local implementation plan is complete; only explicitly scoped fan-in or integration work remains
unblock_condition: explicit Orchestrator fan-in/integration prompt within Common ownership
