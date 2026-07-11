# Control State

component: cli
branch: component/cli
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: none

wave: W1
phase: CLI-P6-BLOCKED-BY-RUNTIME-FAN-IN

implementation_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
ci_workflow: Component CI
ci_run_id: 29084418994
ci_run_number: 1047
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT
resolved_dependencies:
- GDA-P8C lifecycle accepted with green post-fix CI
- Storage component-local lifecycle accepted
- Server Storage feature-isolation fan-in clean accepted
- WT-P9 implementation and post-fix CI accepted
blocker: WT-P9C clean review plus accepted Server/API bootstrap, sync operator, progress/result, and runtime composition contracts are unavailable
unblock_condition: WT-P9C CLEAN_ACCEPT with green final code-bearing CI plus explicit Server/API runtime fan-in or a dedicated CLI fan-in contract
