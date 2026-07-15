# Control State

component: obsidian-plugin
repository: NordCoder/haze-sync
branch: component/obsidian-plugin
status: ACCEPTED_HOLD
repository_access_verified: yes
control_ref_source: component/obsidian-plugin
default_branch_control_is_active: no

active_prompt: apps/haze-obsidian-plugin/control/prompt.md
active_report: none
active_agent_role: orchestrator-hold
assigned_chat_name: none

wave: W1
phase: OBS-FAN-IN-P1-ACCEPTED-HOLD
implementation_status: COMPLETE_AFTER_REVIEW_FIX
fix_status: FIX_COMPLETE
clean_review_status: CLEAN_ACCEPT
ci_status: CI_GREEN
architect_status: ARCHITECT_ACCEPT_COMPONENT_COMPLETE

accepted_candidate:
- code_bearing_sha: 457f1e4904456f5ddf766791e5250e36a0fd23e6
- clean_review_report_blob: c161701fc58b1b00b61a39192f80388833e3053d
- ci_run_id: 29440659397
- ci_run_number: 2019
- ci_conclusion: success

next_gate:
- explicit Orchestrator release-hardening, packaging or later cross-component fan-in prompt
