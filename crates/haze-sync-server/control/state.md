# Control State

component: server
branch: component/server
status: PROMPT_READY

active_prompt: crates/haze-sync-server/control/prompt.md
active_report: crates/haze-sync-server/control/report.md
active_agent_role: fixer-worker
assigned_chat_name: server — W1 SRV-P7B3 Executor CI Fix
prompt_revision: verified by Orchestrator after executor clean-review correction exact-SHA CI failure and diagnostics artifact publication

wave: W1
phase: SRV-P7B3-EXECUTOR-CI-FIX

implementation_status: SELF_ACCEPT
fix_status: NOT_STARTED
clean_review_status: CLEAN_ACCEPT_PENDING_CI_INVALIDATED_BY_FAILURE
architect_status: ARCHITECT_CHANGED_CONTRACTS
ci_status: CI_RED_DIAGNOSTICS_AVAILABLE
known_failed_checks:
- diagnostics finalizer

failed_candidate:
- implementation_candidate_sha: a08739cc8146b4f224475c83fa0652b60782db82
- clean_review_correction_sha: 784a45f879f13914a1732b8ea071ea8f281d6721
- clean_review_report_commit: c00f8d4c128f6770ad3062b6d753e7310299faf0
- clean_review_report_blob: 9bafafc37d0fb789687aedb1920d8b4c52c2eb45
- clean_review_report_status: CLEAN_ACCEPT_PENDING_CI
- post_candidate_changes_before_rotation: report-only

failed_ci_evidence:
- workflow: Component CI
- run_id: 29255656933
- run_number: 1859
- run_attempt: 1
- head_sha: 784a45f879f13914a1732b8ea071ea8f281d6721
- conclusion: failure
- rust_workspace_job: failure
- cargo_fmt: success
- cargo_check: success
- cargo_test: success
- cargo_clippy: success
- diagnostics_finalizer: failure
- diagnostics_upload: success

artifact_evidence:
- artifact_id: 8281161500
- artifact_name: ci-diag__component-server__wf-component-ci__run-29255656933__attempt-1
- artifact_digest: sha256:b005e355fd1cef4f4df37ee0b13e5e6156964eaa79b6fdc46bbe9a8e3a1cc2dc
- artifact_expired: false
- artifact_expires_at: 2026-07-14T13:56:33Z
- artifact_head_sha: 784a45f879f13914a1732b8ea071ea8f281d6721

accepted_owner_shas:
- worktree: 1942946331e8362f19907ab6ad4eb779da70fd57
- storage: 66b6a1f554aae1d1b774cc88560d46dd140c7a54
- server_application_services: 647dce7b624d67663632808906896cb6745ea7e7

archived_completed_slot:
- prompt_index: crates/haze-sync-server/control/log/20260713-140000Z-W1-SRV-P7B3-EXECUTOR-CLEAN-clean-code-reviewer-prompt.md
- report_index: crates/haze-sync-server/control/log/20260713-140000Z-W1-SRV-P7B3-EXECUTOR-CLEAN-clean-code-reviewer-report.md
- prompt_blob: d25ebd04062404cb49169b3f56ffc1f21951d61c
- report_blob: 9bafafc37d0fb789687aedb1920d8b4c52c2eb45
- report_commit: c00f8d4c128f6770ad3062b6d753e7310299faf0

fix_requirements:
- download exact diagnostics artifact 8281161500
- read summary, manifest and every failed_checks log
- identify exact root cause without wrapper-summary guessing
- change only artifact-proven Server-owned scope
- preserve accepted owner snapshots and clean-review validation intent
- produce exact final code-bearing fix SHA
- obtain authoritative DB-capable Component CI on exact fix SHA
- write committed FIX report

completion_requirements:
- committed crates/haze-sync-server/control/report.md exists
- report phase SRV-P7B3-EXECUTOR-CI-FIX
- report chat name server — W1 SRV-P7B3 Executor CI Fix
- report records artifact evidence and diagnosed root cause
- exact final code-bearing SHA and CI result are recorded
- no later product/tooling commit invalidates the fix

next_gate_after_fix:
- FIX_COMPLETE plus green exact-SHA CI -> Orchestrator rotates a new mandatory executor clean-code review
- red CI -> continue fixer routing from the new exact diagnostics artifact
- blocked artifact/contract/scope/tooling -> preserve SRV-P7B4 block and route exact evidence
