# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime
prompt_revision: verified by Orchestrator after WT-P11 exact-SHA fan-in CLEAN_ACCEPT and green DB-capable CI

wave: W1
phase: SRV-P7B4-HOSTED-WORKTREE-RUNTIME

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED_OWNER_BLOCKER_RESOLVED
ci_status: NOT_RUN
known_failed_checks: []

accepted_server_baseline:
- bounded_executor_sha: f8475af72b3e1795c5b11fa39f4625191eff59b1
- bounded_executor_clean_report_commit: b9e22533091a9477876b91729c04682266a1b718
- bounded_executor_clean_status: CLEAN_ACCEPT

accepted_worktree_integration:
- owner_source_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- server_fan_in_sha: 1b2b572a1a200f2968d005e48e9c0674f9db8bc0
- fan_in_clean_report_commit: 0d2d6038fb84805e3844cd539c88fcb409a36cfc
- fan_in_clean_report_blob: ef03533a379c842d93beb4dc072958978da2991a
- fan_in_clean_status: CLEAN_ACCEPT
- ci_run_id: 29275250064
- ci_run_number: 1882
- ci_conclusion: success
- db_capable: yes

resolved_blocker:
- production path-free watcher lifecycle is integrated
- scheduler-accounted host-facing manual request boundary is integrated
- typed Busy/cancellation/lifecycle outcomes are integrated
- cancellation-by-drop safety is integrated
- no further owner change required before SRV-P7B4

phase_goal:
- implement exactly one joined cancellable Server-hosted Worktree runtime task
- compose accepted ProductionWorktreeWatcher, WorktreeHostedRuntime and ServerWorktreeCycleExecutor
- add safe validated host config and bounded manual boundary
- preserve Worktree ownership of scheduling/no-overlap/accounting
- provide secret-safe internal status and focused lifecycle tests

protected_scope:
- accepted Worktree and Storage source
- migrations/schema
- Core/API/CLI/Deployment product files
- public DTO/routes/readiness surfaces
- sibling control files and workflows

completion_requirements:
- real code-bearing Server implementation commit
- exact final SHA recorded
- authoritative DB-capable exact-SHA Component CI green or honest blocker
- committed IMPLEMENTATION report exists

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> focused functional clean review
- substantive defect or red CI -> focused fixer
- formatting/style alone is non-blocking
- API-P8 and SRV-P7B5 remain blocked until SRV-P7B4 CLEAN_ACCEPT
