# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7B2 Follow-Up CI Fix
prompt_revision: verified by Orchestrator after FIX_NEEDS_MORE and independent new-artifact metadata validation

wave: W1
phase: FIX-SRV-P7B2-FOLLOWUP-CI

implementation_status: SELF_ACCEPT_PENDING_CI
fix_status: FIX_NEEDS_MORE
clean_review_status: NOT_STARTED
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED
ci_workflow: Component CI
ci_code_bearing_tooling_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585
ci_run_id: 29172398405
ci_run_number: 1832
ci_run_attempt: 1
known_failed_checks:
- Rust workspace / Finalize CI diagnostics

accepted_srv_p7b2_implementation_sha: 81e6f6ae69f4bda92d284991e4909457a6ef8e60
post_source_fix_sha: 9304263f4be8234e513bf335dc886f96c53d0cce
postgres_tooling_sha: d48bbd847c8b79511a7ac32cfcb14c671f3880c1
helper_sync_merge_commit: 716bd357f52831d796c7e1ea57c83a2849e6ce03
first_post_sync_artifact_id: 8253726096
first_artifact_fix_final_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585
pr_number: 45
pr_status: OPEN_DRAFT_UNMERGED_MERGEABLE

previous_artifact_proven_fixes:
- workspace tests serialized with --test-threads=1
- multi-command conflict fixture replaced by one parameterized CASE update
- test-only process-local async database lease added
- schema probing and one-time migration setup added
- shared DB tables cleaned while lease is held before each DB test
- application/delete/v1/conflict DB tests use the lease
- production behavior and public API unchanged

visible_post_fix_ci_evidence:
- PostgreSQL initialization succeeded
- cargo fmt wrapper succeeded
- cargo check wrapper succeeded
- cargo test wrapper succeeded
- cargo clippy wrapper succeeded
- diagnostics finalizer failed
- diagnostics artifact upload succeeded

artifact_name: ci-diag__component-server__wf-component-ci__run-29172398405__attempt-1
artifact_id: 8253899359
artifact_head_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585
artifact_digest: sha256:a5500f9acca3e1da247943e3b9a8aaab2843891dc7ba83a301a7e928a3b5c4bf
artifact_size_bytes: 8545
artifact_expires_at: 2026-07-12T23:38:53Z
artifact_status: AVAILABLE_UNEXPIRED

current_fix_scope:
- read exact artifact 8253899359
- identify exact remaining underlying failure
- preserve previous DB-test lease and serialization corrections
- change only artifact-proven Server test harness/tests or CI diagnostics integration
- require fully green DB-capable Component CI

parallel_owner_status:
- WT-P10 is CLEAN_ACCEPT at 1942946331e8362f19907ab6ad4eb779da70fd57
- STOR-P10 requires artifact fixer 8253874582 after CLEAN_NEEDS_FIX

blocked_downstream:
- SRV-P7B2 clean-code review remains blocked until FIX_COMPLETE and green DB-capable CI
- SRV-P7B3 remains blocked until WT-P10, STOR-P10 and SRV-P7B2 all reach CLEAN_ACCEPT and exact accepted SHAs are synchronized
- SRV-P7B4, API-P8, SRV-P7B5, CLI-P6A and DEP-P5A remain blocked

next_gate_after_fix:
- FIX_COMPLETE plus green DB-capable CI -> mandatory SRV-P7B2 clean-code review
- FIX_NEEDS_MORE or red CI -> exact next artifact routing
- blocked status -> route exact logs/scope/contract/tooling decision