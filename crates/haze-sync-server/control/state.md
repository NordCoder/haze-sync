# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7B2 CI Fix
prompt_revision: verified by Orchestrator after SRV-P7B2 implementation report and exact diagnostics artifact metadata retrieval

wave: W1
phase: FIX-SRV-P7B2-CI

implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_sha: 81e6f6ae69f4bda92d284991e4909457a6ef8e60
ci_run_id: 29166317284
ci_run_number: 1766
ci_run_attempt: 1
known_failed_checks:
- Finalize CI diagnostics

artifact_name: ci-diag__component-server__wf-component-ci__run-29166317284__attempt-1
artifact_id: 8252253613
artifact_head_sha: 81e6f6ae69f4bda92d284991e4909457a6ef8e60
artifact_digest: sha256:f5e0a2f8dba439b43ca6329b9bd4c3e08292a2e6d50fa2686f59b5ba8d2e87c1
artifact_size_bytes: 14020
artifact_expires_at: 2026-07-12T20:04:08Z
artifact_status: AVAILABLE_UNEXPIRED

accepted_implementation:
- reusable async application services for file, delete, changes and revision-content operations
- route delegation with public behavior parity
- shared transaction, locking, idempotency, Core, object-store and Storage choreography
- deterministic secret-safe future Worktree idempotency derivation
- no Worktree executor, host, schema, public DTO or config work

parallel_owner_status:
- WT-P10 is independently in tooling correction
- STOR-P10 is independently in tooling correction and later requires DB-capable verification

blocked_downstream:
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and exact accepted SHAs are synchronized
- SRV-P7B4, API-P8, SRV-P7B5, CLI-P6A and DEP-P5A remain blocked

next_gate_after_fix:
- FIX_COMPLETE plus independently verified green post-fix code-bearing Component CI -> mandatory SRV-P7B2 clean-code review
- failed or blocked fix -> route exact artifact/scope/contract/tooling decision; no clean review
