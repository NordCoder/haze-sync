# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: implementation-worker
assigned_chat_name: server — W1 SRV-P7B2 Application Services
prompt_revision: verified by Orchestrator after ARCH-SRV-P7B-CONTRACTS accepted reusable async Server application services

wave: W1
phase: SRV-P7B2

implementation_status: NOT_STARTED
fix_status: NOT_REQUIRED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: NOT_RUN

accepted_srv_p7a_code_bearing_sha: 37706634fd8dd2d9b299a1c453718f2de63981d0
accepted_srv_p7a_ci_run: 29161721748
accepted_srv_p7a_ci_run_number: 1704
accepted_srv_p7a_ci_status: CI_GREEN
accepted_srv_p7a_clean_review_status: CLEAN_ACCEPT

architecture_docs_sha: b98f5079ed90ccf7eaec79e617ae591e0c308ff4
architecture_report_commit: 22b5e993431a1c0a858716ac835a3ece8ade6d5c
architecture_ci_run: 29165123142
architecture_ci_run_number: 1719
architecture_status: ARCHITECT_CHANGED_CONTRACTS
architecture_report_archive: crates/haze-sync-server/control/log/20260711-191500Z-W1-ARCH-SRV-P7B-CONTRACTS-architect-report.md

phase_scope:
- reusable async Server application services shared by HTTP routes and future Worktree execution
- file create/update transaction service
- guarded delete/tombstone transaction service
- bounded authoritative changes service
- revision metadata/content retrieval service
- internal actor and deterministic future Worktree idempotency derivation
- route delegation with exact public behavior parity

ownership_invariants:
- routes own transport parsing/auth/API DTO/status mapping only
- Server application services own transaction/lock/idempotency/object-store/repository choreography
- Core remains policy owner
- Storage remains passive and caller-transaction-owned
- no Worktree executor, host, scheduler, schema or public DTO work in SRV-P7B2

parallel_owner_phases:
- WT-P10 may run independently on component/worktree
- STOR-P10 may run independently on component/storage

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 are all CLEAN_ACCEPT and exact accepted SHAs are synchronized
- SRV-P7B4, API-P8, SRV-P7B5, CLI-P6A and DEP-P5A remain blocked by the architect-defined order

next_gate_after_implementation:
- SELF_ACCEPT plus green code-bearing Component CI including mandatory DB-backed parity tests -> mandatory SRV-P7B2 clean-code review
- red CI -> exact artifact-based fixer after Orchestrator metadata retrieval
- blocked status -> route exact contract/scope/tooling decision; no clean review
