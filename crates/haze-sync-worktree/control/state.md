# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: worktree — W1 WT-P11 Cancellation Safety Fix
prompt_revision: verified by Orchestrator after final functional review found one substantive cancellation-by-drop defect; formatting/style remain non-blocking

wave: W1
phase: WT-P11-CANCELLATION-FIX

implementation_status: SELF_ACCEPT_AFTER_FIX
fix_status: NOT_STARTED
clean_review_status: CLEAN_NEEDS_FIX
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: CI_GREEN_FOR_REVIEWED_SHA
known_failed_checks: []

reviewed_candidate:
- code_bearing_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
- functional_review_report_commit: 7e761ccf1b3ba0036d10067a9d6e4bf395db4bf1
- functional_review_report_blob: db35ddba7c77518f6c04216a52907634b451eeef
- review_status: CLEAN_NEEDS_FIX
- sole_blocker: cancellation-by-drop can wedge shared busy gate and close manual ticket without typed cancellation
- post_candidate_product_or_tooling_changes: none

ci_evidence:
- workflow: Component CI
- run_id: 29269422217
- run_number: 1876
- head_sha: 9cce5f5a34597f13a506fadad49a7ab46972fa98
- conclusion: success

required_fix:
- RAII busy/in-flight guard releases gate on normal completion and Drop
- dropped manual execution completes ticket with typed cancellation
- dropped incomplete cycle does not commit completed-cycle accounting
- deterministic pending manual and automatic drop tests
- no stylistic or formatting-only work

completion_requirements:
- real code-bearing cancellation fix commit
- exact final SHA recorded
- exact-SHA Component CI green or honest blocker
- committed FIX report exists

next_gate_after_fix:
- FIX_COMPLETE plus green exact-SHA CI -> final lightweight functional verification, then exact-SHA Server fan-in
- formatting/style does not block
- Server SRV-P7B4 remains blocked until fan-in acceptance
