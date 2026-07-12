# Control State

component: cli
branch: component/cli
status: BLOCKED_BY_DEPENDENCY

active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: none
assigned_chat_name: none
prompt_revision: verified by Orchestrator during full component/dependency audit

wave: W1
phase: CLI-P6A-BLOCKED-BY-API-SERVER

implementation_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
ci_run_id: 29084418994
ci_run_number: 1047
ci_run_attempt: 1
known_failed_checks: []
architect_status: ARCHITECT_ACCEPT

resolved_dependencies:
- WT-P10 CLEAN_ACCEPT
- STOR-P10 CLEAN_ACCEPT
- SRV-P7B2 CLEAN_ACCEPT
- GDrive Adapter component-local lifecycle accepted

remaining_blockers:
- Server exact-SHA fan-in not yet clean-accepted
- SRV-P7B3 bounded executor unavailable
- SRV-P7B4 hosted runtime unavailable
- API-P8 passive status/operator DTO contract unavailable
- SRV-P7B5 safe status/readiness/manual-cycle surface unavailable

next_planned_phase:
- CLI-P6A Worktree status and explicit sync-once operator commands

forbidden_early_work:
- local runtime hosting
- direct DB or filesystem mutation
- provider calls
- hidden daemon ownership
- invented API/status semantics
- destructive repair

branch_precondition:
- explicit synchronization with current main before future CLI-P6A code work

unblock_condition:
- API-P8 and SRV-P7B5 CLEAN_ACCEPT and synchronized
- explicit Orchestrator CLI-P6A prompt pinned to accepted Server/API SHAs