# Control State

component: storage
branch: component/storage
status: PROMPT_READY

active_prompt: crates/haze-sync-storage/control/prompt.md
active_report: crates/haze-sync-storage/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: storage — W1 STOR-P10 Worktree Durable State
prompt_revision: verified by Orchestrator after ARCH-SRV-P7B-CONTRACTS assigned durable Worktree state and cursor ownership to Storage

wave: W1
phase: STOR-P10

implementation_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: NOT_RUN
architect_status: ARCHITECT_CHANGED_CONTRACTS

accepted_baseline_code_bearing_sha: aa59064d641f4850f7c70fa615e638b52613dd95
accepted_baseline_ci_run: 29124956486
accepted_baseline_ci_run_number: 1580
accepted_baseline_ci_status: CI_GREEN
external_server_feature_isolation_status: CLEAN_ACCEPT

architecture_source_branch: component/server
architecture_docs_sha: b98f5079ed90ccf7eaec79e617ae591e0c308ff4
architecture_report_commit: 22b5e993431a1c0a858716ac835a3ece8ade6d5c
architecture_ci_run: 29165123142
architecture_ci_run_number: 1719
architecture_status: ARCHITECT_CHANGED_CONTRACTS

phase_scope:
- versioned Worktree adapter-instance binding keyed by adapter_id
- non-public normalized-root fingerprint verification
- per-instance present/tombstoned path state and bounded versioned reconciliation fields
- caller-transaction-owned path-state repositories
- locked exact-contiguous monotonic adapter_cursors.last_core_seq operations
- migration, legacy compatibility and crash/rollback tests

ownership_invariants:
- Storage owns schema, models and passive repository primitives
- Server will own transaction timing and runtime composition
- Worktree owns filesystem and semantic planning/materialization behavior
- Core/API policy and public DTO ownership remain unchanged

parallel_owner_phases:
- WT-P10 may run independently on component/worktree
- SRV-P7B2 may run independently on component/server

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 are all CLEAN_ACCEPT with exact accepted SHAs synchronized
- Deployment migration work remains blocked until STOR-P10 clean acceptance and later Server runtime phases

next_gate_after_implementation:
- SELF_ACCEPT plus green code-bearing Component CI and mandatory DB/migration evidence -> STOR-P10 clean-code review
- red CI -> exact artifact-based fixer after Orchestrator metadata retrieval
- blocked status -> route exact contract/scope/tooling decision; no clean review
