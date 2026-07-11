# Control State

component: worktree
branch: component/worktree
status: PROMPT_READY

active_prompt: crates/haze-sync-worktree/control/prompt.md
active_report: crates/haze-sync-worktree/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: worktree — W1 WT-P10 Async Runtime Contract
prompt_revision: verified by Orchestrator after ARCH-SRV-P7B-CONTRACTS accepted the awaitable Worktree cycle architecture

wave: W1
phase: WT-P10

implementation_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_CHANGED_CONTRACTS

accepted_baseline_code_bearing_sha: 4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
accepted_baseline_ci_run: 29152965199
accepted_baseline_ci_run_number: 1665
accepted_baseline_ci_status: CI_GREEN

architecture_source_branch: component/server
architecture_docs_sha: b98f5079ed90ccf7eaec79e617ae591e0c308ff4
architecture_report_commit: 22b5e993431a1c0a858716ac835a3ece8ade6d5c
architecture_ci_run: 29165123142
architecture_ci_run_number: 1719
architecture_status: ARCHITECT_CHANGED_CONTRACTS

phase_scope:
- Worktree-owned awaitable WorktreeRuntimeCycle contract
- async-compatible WorktreeRuntimeService poll awaiting at most one cycle
- explicit cooperative cancellation observation
- explicit DryRun representation and non-mutation semantics
- preserve scheduler full-scan, watcher, budget, summary validation and no-overlap invariants
- synchronous filesystem primitives remain unchanged

forbidden_workarounds:
- nested Tokio runtime or Handle::block_on
- ad hoc blocking of async authority
- detached or hidden tasks
- Server/Storage/database/HTTP behavior
- fake summaries or production substitutes

parallel_owner_phases:
- STOR-P10 may run independently on component/storage
- SRV-P7B2 may run independently on component/server

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 are all CLEAN_ACCEPT with exact accepted SHAs synchronized

next_gate_after_implementation:
- SELF_ACCEPT plus green code-bearing Component CI -> mandatory WT-P10 clean-code review
- red CI -> exact artifact-based fixer after Orchestrator metadata retrieval
- blocked status -> route exact contract/scope/tooling decision; no clean review
