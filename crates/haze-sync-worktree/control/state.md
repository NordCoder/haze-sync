# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: clean-code-reviewer
assigned_chat_name: worktree — W1 WT-P12 Manual Status Review
prompt_revision: verified by Orchestrator after SELF_ACCEPT owner extension and green exact-SHA CI; formatting/style excluded from blocking scope

wave: W1
phase: WT-P12-FUNCTIONAL-REVIEW

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED_FUNCTIONAL
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN
known_failed_checks: []

candidate:
- accepted_wt_p11_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- final_code_bearing_sha: 526714cdfe185713a09af68fd5bddcb967a7902e
- implementation_report_commit: f7ad06d4ddec016c35555388634926bda5f963b0
- implementation_report_blob: a561a094346abbdf58df88d1da51b28ba93aac69
- post_candidate_changes: report-only before rotation

ci_evidence:
- workflow: Component CI
- run_id: 29287214701
- run_number: 1905
- head_sha: 526714cdfe185713a09af68fd5bddcb967a7902e
- conclusion: success
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: success

review_policy:
- formatting, rustfmt, naming taste and style are non-blocking and out of scope
- verify authoritative shared gate, lifecycle/busy transitions, race safety, passivity, secrecy and protected scope

completion_requirements:
- committed CLEAN_CODE_REVIEW report exists
- report phase WT-P12-FUNCTIONAL-REVIEW
- report chat name worktree — W1 WT-P12 Manual Status Review
- exact final SHA reviewed
- no later product/tooling invalidation

next_gate_after_review:
- CLEAN_ACCEPT -> exact-SHA fan-in to Server and resume SRV-P7B5
- substantive defect -> focused Worktree fix only
- API-P8 remains blocked
