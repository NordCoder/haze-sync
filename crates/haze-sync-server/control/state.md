# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 WT-P11 Exact-SHA Fan-In
prompt_revision: verified by Orchestrator after WT-P11 CLEAN_ACCEPT on exact SHA and green CI

wave: W1
phase: SRV-WT-P11-FAN-IN

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED
ci_status: NOT_RUN
known_failed_checks: []

accepted_worktree_source:
- baseline_sha: 1942946331e8362f19907ab6ad4eb779da70fd57
- accepted_code_bearing_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- clean_report_commit: b76f88369d079a9cb264bb4cdb9b6c62e361a9bc
- clean_report_blob: 84fb7a399d708088c3b545efd0e2adbaad3f909e
- clean_status: CLEAN_ACCEPT
- ci_run_id: 29272964159
- ci_run_number: 1881
- ci_run_attempt: 1
- ci_conclusion: success

accepted_server_baseline:
- bounded_executor_sha: f8475af72b3e1795c5b11fa39f4625191eff59b1
- bounded_executor_clean_report_commit: b9e22533091a9477876b91729c04682266a1b718
- bounded_executor_clean_status: CLEAN_ACCEPT

phase_goal:
- copy only exact WT-P11 Worktree product/dependency changes into component/server
- preserve Server-owned work and unrelated accepted snapshots
- do not copy Worktree control/log files
- obtain DB-capable exact-SHA Server Component CI
- prepare for functional integration review

allowed_scope:
- exact accepted WT-P11 Worktree product files and required lockfile changes
- Server control report
- narrow conflict resolution required by prior accepted fan-in

protected_scope:
- Server product semantics outside fan-in conflict resolution
- Storage/Core/API/CLI/Deployment product files
- migrations/workflows/sibling control files

completion_requirements:
- real code-bearing Server fan-in commit
- exact source range and copied paths recorded
- final Server code-bearing SHA recorded
- authoritative DB-capable Component CI green or honest blocker
- committed IMPLEMENTATION report exists

next_gate_after_fan_in:
- SELF_ACCEPT plus green exact-SHA CI -> focused functional integration review
- CLEAN_ACCEPT after integration review -> reactivate SRV-P7B4 Hosted Worktree Runtime
- formatting/style alone is non-blocking
