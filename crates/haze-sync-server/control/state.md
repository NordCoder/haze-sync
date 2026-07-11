# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7A Worktree Snapshot Fan-In
prompt_revision: verified by Orchestrator after Worktree WT-P9C final green acceptance

wave: W1
phase: SRV-P7A

implementation_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_ACCEPT

server_baseline_sha: dc53f8dbe08da56d129fc3898cec262149c69f38
server_baseline_ci_run: 29127012776
server_baseline_ci_run_number: 1616

fan_in_source_branch: component/worktree
fan_in_source_sha: 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
fan_in_source_ci_run: 29152965199
fan_in_source_ci_run_number: 1665
fan_in_source_status: CLEAN_ACCEPT_AND_CI_GREEN

branch_gap: component/server currently contains the placeholder Worktree crate and requires explicit accepted-snapshot synchronization
phase_scope: exact Worktree product snapshot fan-in plus Server-owned composition/lifecycle/status boundary; concrete executor loop may be deferred honestly to SRV-P7B
next_gate_after_implementation: mandatory clean-code review, then authoritative Component CI and fixer loop if needed
