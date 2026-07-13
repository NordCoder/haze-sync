# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: clean-code-reviewer
assigned_chat_name: worktree — W1 WT-P11 Cancellation Verification
prompt_revision: verified by Orchestrator after cancellation-by-drop fix and green exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: WT-P11-CANCELLATION-VERIFY

implementation_status: SELF_ACCEPT_AFTER_FIX
fix_status: FIX_COMPLETE
clean_review_status: NOT_STARTED_FINAL_FUNCTIONAL
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN
known_failed_checks: []

final_candidate:
- previous_code_bearing_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
- final_code_bearing_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- fix_report_commit: 952d4d45066ed2fca58db53c20b0ae767ac31221
- fix_report_blob: 30aa9425f0a7738acbe91e6d867e8c20215bec04
- post_candidate_changes: report-only before rotation

ci_evidence:
- workflow: Component CI
- run_id: 29272964159
- run_number: 1881
- run_attempt: 1
- head_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- conclusion: success
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: success

review_policy:
- formatting, rustfmt, naming taste and style are non-blocking and out of scope
- verify only cancellation, lifecycle, accounting, safety and scope invariants

completion_requirements:
- committed CLEAN_CODE_REVIEW report exists
- report phase WT-P11-CANCELLATION-VERIFY
- report chat name worktree — W1 WT-P11 Cancellation Verification
- exact final SHA reviewed
- no later product/tooling invalidation

next_gate_after_review:
- CLEAN_ACCEPT -> exact-SHA Worktree fan-in to component/server immediately
- substantive defect -> focused fix only
