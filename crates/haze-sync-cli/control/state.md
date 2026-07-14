# Control State

component: cli
branch: component/cli
status: ACCEPTED_HOLD
active_prompt: crates/haze-sync-cli/control/prompt.md
active_report: crates/haze-sync-cli/control/report.md
active_agent_role: orchestrator-hold
assigned_chat_name: none

wave: W1
phase: CLI-P6A-ACCEPTED-HOLD
implementation_status: COMPLETE_AFTER_FIX
fix_status: FIX_COMPLETE
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
architect_status: ARCHITECT_ACCEPT
known_failed_checks: []

accepted_candidate:
- final_code_bearing_sha: d33fa105398d9731bfc1b7927e98d5d085c6fe59
- clean_report_blob: 04db6dc72f90ff6ea757312bb31dc616e0dc1bc0
- ci_run_id: 29368943579
- ci_run_number: 1953
- ci_conclusion: success

next_gate:
- Deployment pre-sync may begin
- DEP-P5A product work still requires explicit migration execution and operational ownership policy
