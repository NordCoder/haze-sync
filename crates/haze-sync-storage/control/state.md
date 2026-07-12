# Control State

component: storage
branch: component/storage
status: PROMPT_READY

active_prompt: crates/haze-sync-storage/control/prompt.md
active_report: crates/haze-sync-storage/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: storage — W1 STOR-P10 Clean Correction CI Fix
prompt_revision: verified by Orchestrator after CLEAN_NEEDS_FIX and independent artifact metadata validation

wave: W1
phase: FIX-STOR-P10-CLEAN-CI

implementation_status: SELF_ACCEPT
fix_status: FIX_REQUIRED_AFTER_CLEAN_REVIEW
clean_review_status: CLEAN_NEEDS_FIX
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_tooling_sha: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
ci_run_id: 29172303439
ci_run_number: 1825
ci_run_attempt: 1
known_failed_checks:
- Rust workspace / Finalize CI diagnostics

accepted_pre_phase_sha: aa59064d641f4850f7c70fa615e638b52613dd95
post_fix_storage_sha: abca69058390983894465cef9d66c38960fae4c7
initial_clean_review_candidate_sha: 2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2
clean_review_correction_sha: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
pr_number: 47
pr_status: OPEN_DRAFT_UNMERGED_MERGEABLE

clean_review_correction:
- direct savepoint-backed migration 0010 SQL guard test added
- non-empty legacy row and exact table shape preservation proven
- absence of worktree_instances after guard failure proven
- Storage PostgreSQL evidence gate expanded to five mandatory tests
- production migration SQL and repository API unchanged

visible_ci_evidence:
- Storage PostgreSQL verification succeeded
- PostgreSQL readiness succeeded
- strict command cargo test -p haze-sync-storage --features test-support -- --ignored succeeded
- all five mandatory evidence checks succeeded
- Storage PostgreSQL finalizer succeeded
- ordinary fmt/check/test/clippy wrapper steps succeeded
- ordinary Rust diagnostics finalizer failed
- diagnostics artifact upload succeeded

artifact_name: ci-diag__component-storage__wf-component-ci__run-29172303439__attempt-1
artifact_id: 8253874582
artifact_head_sha: a6f1edf48d23c767f0f9b34ab28aacd8bd586000
artifact_digest: sha256:41c87f30baa40ab384db02ff7884895d2727a94dcb50ea4e5b9b410fd467bf34
artifact_size_bytes: 1824
artifact_expires_at: 2026-07-12T23:34:59Z
artifact_status: AVAILABLE_UNEXPIRED

current_fix_scope:
- read exact artifact 8253874582
- identify exact underlying failed marker despite green wrapper summaries
- make only artifact-proven Storage test or CI diagnostics correction
- preserve fifth migration guard evidence test and all STOR-P10 semantics
- require both ordinary and PostgreSQL jobs fully green

parallel_owner_status:
- WT-P10 is CLEAN_ACCEPT at 1942946331e8362f19907ab6ad4eb779da70fd57
- SRV-P7B2 requires a separate follow-up artifact fixer for artifact 8253899359

blocked_downstream:
- STOR-P10 final clean acceptance remains blocked until FIX_COMPLETE and both CI jobs green
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and accepted SHAs are synchronized
- Deployment migration work remains blocked

next_gate_after_fix:
- FIX_COMPLETE plus both jobs green -> final STOR-P10 clean-code acceptance review
- FIX_NEEDS_MORE or red CI -> exact next artifact routing
- blocked status -> route exact logs/scope/contract/tooling decision