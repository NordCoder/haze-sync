# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: fixer-worker

wave: W1
phase: FIX-WT-P9-CI

implementation_status: BLOCKED_BY_TOOLING
clean_review_status: NOT_STARTED
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: a14c4e953523e14d73431ae54ac03ab42101362c
ci_run_id: 29143968689
ci_run_number: 1655
ci_run_attempt: 1
ci_artifact_name: ci-diag__component-worktree__wf-component-ci__run-29143968689__attempt-1
ci_artifact_id: 8246127652
known_failed_checks:
- Finalize CI diagnostics
architect_status: ARCHITECT_ACCEPT
next_gate_after_fix: clean-code-review for WT-P9 after a verified green post-fix code-bearing CI run
fan_in_note: this fixer pass is Worktree-owned only; concrete Server hosting remains a dedicated later fan-in
