REPORT_TYPE:
FIX

STATUS:
FIX_COMPLETE

AGENT:
role: fixer-worker
agent_execution_id: W1-FIX-API-P4-CI
chat_name: api — W1 FIX-API-P4-CI CI Fix

COMPONENT:
name: api
path: crates/haze-sync-api
branch: component/api
contract_path: crates/haze-sync-api/docs/component-contract.md
plan_path: crates/haze-sync-api/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-api/docs/dependency-map.md
control_prompt_path: crates/haze-sync-api/control/prompt.md
control_report_path: crates/haze-sync-api/control/report.md

WAVE:
id: W1
phase_id: FIX-API-P4-CI
dependency_status: active control state was PROMPT_READY; API-P4 implementation had CI_RED for run 29028332903 attempt 1

SUMMARY:
Fixed the minimum API-P4 CI failures identified by the diagnostics artifact for Component CI run 29028332903 attempt 1. The artifact showed rust-fmt drift in crates/haze-sync-api/src/routes/files.rs and compile failures in integration tests caused by adding a new required adapter_principal field to the public PutFileRouteRequestParts struct. To keep existing file-route contract callers source-compatible while preserving an explicit verified-principal path for Server wiring, the fix removed adapter_principal from PutFileRouteRequestParts, made PutFileRouteRequest carry an optional adapter principal, kept parse_put_file_request as a passive metadata parser, and added parse_authenticated_put_file_request(parts, adapter_principal) as the principal-requiring helper. Applied the rustfmt-required formatting changes in the same file. No tests were deleted, no workflow files were changed, and no Axum handlers, storage/Core calls, object-store reads/writes, operation-log queries, content streaming, or background cursor updates were added.

CHANGED_FILES:
- crates/haze-sync-api/src/routes/files.rs
- crates/haze-sync-api/control/report.md

BRANCH_AND_CONTROL:
current_branch: component/api
base_branch: main
base_sha: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
head_sha: 7748d81690ee1d78035c21756e6bd9235127dadb before report-only commit; final report-only commit follows with CI skip
default_branch_modified: no
sibling_branch_modified: no
control_prompt_read: yes
control_report_written: yes
control_files_archived_by_worker: no
ci_skip_used: yes, only for final report-only commit
ci_skip_reason: final commit updates only crates/haze-sync-api/control/report.md and cannot change executable behavior or validation outcome; skipped workflow is not CI evidence

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
scope_expansion_rationale: none
cross_component_changes: no
forbidden_files_touched: no

CONTRACT:
contract_read: yes
contract_satisfied: yes
contract_changes_requested: none
contract_change_rationale: none
affected_components: none

IMPLEMENTATION_OR_REVIEW:
completed: yes
main_changes:
- Removed the newly added adapter_principal field from public PutFileRouteRequestParts so existing integration contract tests and callers can still construct request parts without a breaking struct-literal change.
- Changed PutFileRouteRequest to store adapter_principal: Option<AdapterPrincipal> and expose adapter_principal() as Option<&AdapterPrincipal>.
- Kept parse_put_file_request as the stable passive PUT metadata parser for path, idempotency key, content hash, base revision, body length, and body bytes.
- Added parse_authenticated_put_file_request(parts, adapter_principal) to require an already verified AdapterPrincipal without adding runtime auth lookup or middleware.
- Updated API-local route-helper tests to cover the authenticated helper's missing-principal 401/missing_token mapping and successful principal attachment.
- Applied rustfmt-required formatting changes reported for rejected_upload_response, parse_required_adapter_principal, parse_required_base_revision, and an upload-response matches assertion.
behavior_changes: parse_put_file_request is restored to source-compatible metadata-only behavior; verified adapter principal requirement is now represented by a separate passive helper instead of a new required public struct field
bugs_found:
- PutFileRouteRequestParts gained a required adapter_principal field, breaking existing integration tests and callers that construct the public request-parts struct.
- routes/files.rs had rustfmt drift.
bugs_fixed:
- Restored source compatibility for PutFileRouteRequestParts construction.
- Added a non-breaking principal-requiring helper for future Server wiring.
- Applied rustfmt-equivalent formatting.
cleanups_made: constrained to artifact-proven formatting and API route-helper compatibility repair
non_goals_preserved:
- no Axum handler implementation
- no object-store reads or writes
- no operation-log queries
- no content streaming
- no background cursor updates
- no runtime auth lookup
- no workflow changes
- no sibling component changes
- no tests deleted
deferred_work:
- Wait for the new Component CI workflow run for the code-bearing fix commit to finish.

TESTS_AND_CHECKS:
checks_run:
- Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, and chatgpt-gh-connector.md from Project Sources.
- Read current control state and active API-P4 fixer prompt from component/api.
- Read previous API-P4 implementation report.
- Read API component contract, implementation plan API-P4 section, implementation log, and dependency map.
- Listed and downloaded diagnostics artifact 8202653799 for workflow run 29028332903 attempt 1.
- Read diagnostic summary.md and manifest.json.
- Read all failed-check logs and failure markers listed in failed_checks: rust-fmt, cargo-check, cargo-test, and cargo-clippy.
- Inspected integration tests that failed to compile: crates/haze-sync-api/tests/file_routes_contract.rs and crates/haze-sync-api/tests/core_safety_contracts.rs.
- Verified changed regions in crates/haze-sync-api/src/routes/files.rs through GitHub connector file reads.
- Observed new Component CI run 29034803865 for code-bearing fix commit 7748d81690ee1d78035c21756e6bd9235127dadb as in_progress.
- Observed the new run's Rust workspace job with cargo fmt completed successfully and cargo check in progress.
checks_not_run:
- cargo fmt --all --check locally
- cargo check --workspace locally
- cargo test --workspace locally
- cargo clippy --workspace --all-targets -- -D warnings locally
Reason: this worker is constrained to GitHub connector only; no local shell execution is available.
ci_status: CI_PENDING
workflow_urls:
- Component CI run 29034803865 observed in_progress for 7748d81690ee1d78035c21756e6bd9235127dadb
known_failures:
- Original failed checks in run 29028332903 attempt 1: rust-fmt, cargo-check, cargo-test, cargo-clippy

CI_DIAGNOSTICS:
artifact_based_logs: yes
artifact_name: ci-diag__component-api__wf-component-ci__run-29028332903__attempt-1
artifact_id: 8202653799
workflow_run_id: 29028332903
workflow_run_attempt: 1
artifact_status: available, not expired, downloaded and readable; diagnostic files were present at artifact root as summary.md, manifest.json, failures/*.txt, and logs/*.log
summary_read: yes, summary.md
manifest_read: yes, manifest.json
logs_read:
- logs/rust-fmt.log
- logs/cargo-check.log
- logs/cargo-test.log
- logs/cargo-clippy.log
- failures/rust-fmt.txt
- failures/cargo-check.txt
- failures/cargo-test.txt
- failures/cargo-clippy.txt
raw_job_logs_used: no
diagnostics_failure: none

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
provider_calls_added: no
hard_delete_added: no
background_jobs_added: no

ISSUES_FOUND:
- Diagnostics artifact paths were at artifact root rather than under ci-diagnostics/, but the required summary, manifest, failure markers, and logs were present and readable.
- New CI is pending for the code-bearing fix commit and was not observed green before writing this report.

BLOCKERS:
none

NEXT_RECOMMENDED_AGENT:
orchestrator

FINAL_VERDICT:
FIX_COMPLETE. Minimum artifact-proven API-P4 CI causes were fixed inside API scope. Post-fix CI is pending and must be observed by Orchestrator.

PUSHED:
yes
