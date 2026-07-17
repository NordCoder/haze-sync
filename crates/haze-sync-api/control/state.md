# Control State

component: api
repository: NordCoder/haze-sync
branch: component/api
status: PROMPT_READY
repository_access_verified: yes
control_ref_source: component/api
default_branch_control_is_active: no

active_prompt: crates/haze-sync-api/control/prompt.md
active_report: crates/haze-sync-api/control/report.md
active_agent_role: fixer-worker
agent_execution_id: api-FIX-API-GDA-P2-ci-diagnostics-20260717130949-ee53d9
chat_key: api

wave: W1
phase: FIX-API-GDA-P2-CI-DIAGNOSTICS
implementation_status: SELF_NEEDS_FIX
fix_status: NOT_STARTED
clean_review_status: NOT_STARTED
ci_status: CI_RED
architect_status: ARCHITECT_CHANGED_CONTRACTS

active_prompt_identity:
- prompt_commit_sha: bcb952e00c01f86aa04f16a841724e5e3bbb165c
- prompt_blob_sha: b0200262283f6d8aa2383a5e9de803458b13f9c2

implementation_candidate:
- previous_accepted_baseline_sha: c60c3976696da1970d539e5cff6e9f74a61fc10e
- implementation_code_bearing_sha: 618fda1d01ec636ba95884f5cf8f6a596560381b
- implementation_report_blob: f60ca2d90b233d541a70451883838dc873d6da49
- implementation_report_status: SELF_NEEDS_FIX
- architect_report_blob: dc95fa55d3b707da462beebe56b32d73cd54db86

ci_failure_evidence:
- workflow: Component CI
- run_id: 29582479954
- run_number: 2066
- run_attempt: 1
- conclusion: failure
- job_id: 87891297074
- diagnostics_finalizer: failure
- artifact_id: 8407621247
- artifact_name: ci-diag__component-api__wf-component-ci__run-29582479954__attempt-1
- artifact_digest: sha256:077d47353cd09a4575a26cf83a7f7962b9cb667411cbb359aeeb08cac4481ba9
- artifact_expired: false

protected_scope:
- artifact-first diagnosis
- minimum API-owned correction only
- no workflow or sibling-component edits
- no test weakening or contract rollback
- no raw cursor, token, provider payload, private path or raw log output
- no merge, rebase, force-push or draft-state changes

next_gate:
- FIX_COMPLETE with exact-SHA full green CI -> focused API clean-code/security review
- FIX_NEEDS_MORE -> evaluate latest exact diagnostics
- blocked status -> route only the proven owner or hold
- Server remains blocked until API fixer and clean review acceptance
