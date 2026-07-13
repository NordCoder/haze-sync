# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: worktree — W1 WT-P12 Manual Status Contract
prompt_revision: verified by Orchestrator after downstream SRV-P7B5 contract blocker; formatting/style excluded from blocking scope

wave: W1
phase: WT-P12-MANUAL-STATUS-CONTRACT

implementation_status: NOT_STARTED
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_NOT_REQUIRED_MINIMAL_OWNER_EXTENSION
ci_status: NOT_RUN
known_failed_checks: []

accepted_baseline:
- wt_p11_code_bearing_sha: b38264ce2b09632a4c0bab0dd77319e1db239a3b
- clean_report_commit: b76f88369d079a9cb264bb4cdb9b6c62e361a9bc
- clean_report_blob: 84fb7a399d708088c3b545efd0e2adbaad3f909e
- clean_status: CLEAN_ACCEPT
- ci_run_id: 29272964159
- ci_run_number: 1881
- ci_conclusion: success

owner_extension_goal:
- add authoritative passive manual lifecycle/busy status over existing gate
- preserve Worktree ownership of submission, Busy, no-overlap, completion and cancellation
- no duplicate gate, request probe, task, poller or runtime
- coarse secret-safe fields only

downstream_blocker:
- Server mirror can report false Busy during idle DryRun polling
- older completion can clear newer accepted request
- accepted contract currently has no passive lifecycle/busy view or completion identity

protected_scope:
- Server, Storage, Core, API, CLI and Deployment product files
- migrations/schema/workflows
- public HTTP/DTO surfaces
- payload/path/backend details
- unrelated cleanup

completion_requirements:
- real code-bearing Worktree commit
- exact final SHA recorded
- authoritative exact-SHA Component CI green or honest blocker
- committed IMPLEMENTATION report exists

next_gate_after_implementation:
- SELF_ACCEPT plus green exact-SHA CI -> focused functional clean review
- CLEAN_ACCEPT -> exact-SHA fan-in to Server and resume SRV-P7B5
- API-P8 remains blocked
