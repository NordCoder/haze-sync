REPORT_TYPE: FIX

STATUS: FIX_NEEDS_MORE

AGENT:
role: fixer-worker
agent_execution_id: W1-FIX-SRV-P7B2-POST-SYNC-CI-server-fixer
chat_name: server — W1 SRV-P7B2 Post-Sync CI Fix

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: FIX-SRV-P7B2-POST-SYNC-CI

SUMMARY:
Read and verified the exact authorized diagnostics artifact from run 29171731875. The finalizer was not stale: the artifact recorded `cargo test --workspace` exit 101. Six Server tests shared one PostgreSQL database, raced non-idempotent migration/table cleanup, and one conflict fixture submitted two SQL commands through one prepared statement. Applied the minimum test/tooling corrections: serialized the workspace test harness, replaced the multi-command fixture update with one CASE-based prepared statement, and added a test-only Server database lease that serializes DB tests, probes/applies the schema once, and cleans tables before each test. The post-fix Component CI run still ended red at the diagnostics finalizer and produced a new artifact. The active prompt authorizes only the prior artifact, so the new artifact was not opened or interpreted.

AUTHORIZED_ARTIFACT:
artifact_name: ci-diag__component-server__wf-component-ci__run-29171731875__attempt-1
artifact_id: 8253726096
artifact_digest_expected: sha256:6d12f4a08e3fcc69a42252dbc9cd8b7c135aab68cbed5bb64d6cb76550058c9b
artifact_digest_verified: yes
artifact_head_sha_expected: 56a0c8835e6d5ba33696b98814c0ac7b475c9b9e
artifact_head_sha_verified: yes
files_read:
- summary.md
- manifest.json
- failures/cargo-test.txt
- logs/cargo-test.log
raw_job_logs_used: no
failed_check: cargo-test
exit_code: 101
artifact_proven_failures:
- application::tests::application_services_preserve_atomic_file_delete_and_read_semantics
- routes::conflicts::tests::metadata_only_resolution_actions_persist_resolution_without_mutating_current_file_when_real_postgres_is_available
- routes::delete::tests::delegated_delete_route_preserves_tombstone_replay_and_stale_base_behavior
- routes::conflicts::tests::resolved_conflicts_cannot_be_resolved_twice_when_real_postgres_is_available
- routes::v1::tests::delegated_routes_preserve_put_get_and_changes_wire_behavior
- routes::conflicts::tests::accept_conflict_stays_not_implemented_and_does_not_mutate_state_when_real_postgres_is_available

CHANGED_FILES:
- .github/workflows/component-ci.yml
- crates/haze-sync-server/src/application/mod.rs
- crates/haze-sync-server/src/application/test_db.rs
- crates/haze-sync-server/src/application/tests.rs
- crates/haze-sync-server/src/routes/delete/tests.rs
- crates/haze-sync-server/src/routes/v1/tests.rs
- crates/haze-sync-server/src/routes/conflicts/tests.rs
- crates/haze-sync-server/control/report.md

FIXES:
- changed ordinary workspace test invocation to `cargo test --workspace -- --test-threads=1`
- added one process-local async mutex for the shared Server PostgreSQL test database
- added safe schema probing through the final migration table before applying non-idempotent migrations
- retained strict required-database behavior for the three SRV-P7B2 parity tests
- retained optional local behavior for existing conflict integration tests
- cleaned shared tables while holding the lease before each DB-backed test
- added explicit reset between the three actions exercised by the metadata-only conflict test
- replaced two semicolon-separated UPDATE commands with one parameterized `UPDATE ... CASE`
- did not modify product runtime behavior, public routes, DTOs, schemas, migrations, Storage, Worktree, Core, API, Common, CLI, or Deployment

CODE_BEARING_COMMITS:
- 384e3926fd8fb9c27d6fa10dea481766cf939f9a serialize workspace tests
- a74546db5fa8ae7af45a6b5f9d86d2e1bcbef73c single prepared conflict-head update
- 65ff0c6175de499fe1ab2e0f7b0afb6a4b15d120 initial shared DB lease
- 01206b1664458d3c5a9a8b2be90f7bdaeace25d0 expose test-only lease
- 2b5d3911e15e3cd32415744aaf02394b0ec78224 application parity lease
- 08ca7e312161528284f079972fe2c7b07d187726 delete route parity lease
- ea520fe2e01b9148d90b6f498eea4ab312ae3df6 v1 route parity lease
- 013fec1f82b59246ce0686e8cc337c0963ef0174 resettable shared DB lease
- 4ecd69cd7e65b40b71374032718b5d52ab0fc585 conflict route leases
final_code_bearing_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585

POST_FIX_CI:
workflow: Component CI
run_id: 29172398405
run_number: 1832
attempt: 1
head_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585
status: completed
conclusion: failure
postgres_container_initialization: success
cargo_fmt_wrapper: success
cargo_check_wrapper: success
cargo_test_wrapper: success
cargo_clippy_wrapper: success
finalize_ci_diagnostics: failure
diagnostics_upload: success

NEW_DIAGNOSTICS_ARTIFACT:
artifact_name: ci-diag__component-server__wf-component-ci__run-29172398405__attempt-1
artifact_id: 8253899359
artifact_digest: sha256:a5500f9acca3e1da247943e3b9a8aaab2843891dc7ba83a301a7e928a3b5c4bf
artifact_size_bytes: 8545
artifact_head_sha: 4ecd69cd7e65b40b71374032718b5d52ab0fc585
artifact_status: available and unexpired
artifact_read: no
reason_not_read: active prompt explicitly authorized only artifact 8253726096

PRESERVATION:
application_service_authority_preserved: yes
routes_remain_transport_adapters: yes
postgresql_service_preserved: yes
mandatory_tests_deleted_ignored_or_weakened: no
public_behavior_changed: no
secrets_committed: no
raw_paths_or_database_errors_exposed: no
current_main_synchronization_preserved: yes
temporary_write_workflow_added: no

CI_SKIP:
used: yes
reason: final report-only control commit; all test/tooling corrections triggered ordinary CI
report_commit_is_ci_evidence: no

NEXT_RECOMMENDED_AGENT:
fixer-worker after Orchestrator assigns exact artifact 8253899359

FINAL_VERDICT:
FIX_NEEDS_MORE. The original artifact-proven failures were addressed without weakening tests, but the new code-bearing run remains red. SRV-P7B2 is not ready for clean-code review until artifact 8253899359 is explicitly assigned, read, minimally corrected, and followed by a fully green DB-capable Component CI run.

PUSHED:
yes
