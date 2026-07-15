REPORT_TYPE: CLEAN_CODE_REVIEW

STATUS: CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: api-gda-p1-clean-review-20260715
chat_name: api — W1 API-GDA-P1 Clean Review

COMPONENT:
name: api
path: crates/haze-sync-api
branch: component/api
control_prompt_path: crates/haze-sync-api/control/prompt.md
control_report_path: crates/haze-sync-api/control/report.md

WAVE:
id: W1
phase_id: API-GDA-P1-CLEAN-REVIEW

CANDIDATE:
reviewed_code_bearing_sha: 77945118a37c6e8efec04ecd054e0e5a5e4435ba
implementation_report_blob: 490921fb423bb0c6119bb966cab49f72d6f0e619
fixer_report_blob: 5b91145a8c0e8746476e3bd5b058e2fcb437f517
accepted_storage_gdrive_sha: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
storage_clean_report_blob: 4584b8705221d3cd2aa43b5776674b3a1ec9a0f4
architecture_report_blob: 14c427880e1201d851cdc9ee04b9cd0e83334de4

SUMMARY:
Reviewed the exact API-GDA-P1 candidate for contract compatibility, authorization, idempotency metadata, CAS/cursor semantics, collection/string/numeric bounds, fixture determinism, public/admin sanitization, secrecy and passivity. One substantive secrecy defect requires a focused API fix before Server GDrive application/transaction fan-in is unblocked. The accepted private wire contract and synthetic fixture are otherwise coherent, exact-SHA CI is green, and no runtime or sibling-component boundary violation was found.

BLOCKING_FINDING:
id: API-GDA-P1-DEBUG-PROVIDER-FACT-LEAK
severity: blocking
affected_paths:
- crates/haze-sync-api/src/dto/gdrive.rs
- crates/haze-sync-api/src/routes/gdrive.rs
- crates/haze-sync-api/tests/gdrive_state_compatibility_fixture.rs
finding:
- GDriveMappingFactsDto derives Debug while containing unwrapped provider/path facts including path, drive_name, mime_type, md5_checksum, drive_modified_time, core_object_id, core_revision_id and timestamps.
- GDriveStateSnapshotResponse and GDriveStateCommitRequest also derive Debug and recursively expose those mapping facts.
- AuthenticatedGDriveStateCommitRequest has a manual Debug implementation that includes `.field("body", &self.body)`, so formatting the authenticated request reaches the derived Debug implementation of the full commit body.
- Existing tests prove redaction only for wrapped drive_file_id, drive_version, facts_fingerprint and Idempotency-Key. They do not assert that drive_name, MIME, checksum, paths, Core identifiers or timestamps are absent.
why_blocking:
- The active review contract explicitly requires raw cursor, provider fact and idempotency redaction from Debug/Display/errors/public-admin output/fixtures.
- Accepted Storage clean evidence treats provider facts as omitted or redacted from Debug/Display.
- The current implementation protects opaque wrapper values but leaves the rest of the mapping/provider fact bundle loggable through derived Debug.
minimum_fix:
- Replace derived Debug on private mapping/snapshot/commit fact DTOs with explicit secret-safe implementations, or remove Debug where not required.
- Ensure AuthenticatedGDriveStateCommitRequest does not recursively format the private body; use a fixed marker or a deliberately sanitized body summary.
- Add tests using unique sentinel values for path, drive_name, MIME, checksum, Core IDs and timestamps and assert none appear in Debug output.
- Preserve serde wire shapes, fixture JSON, authorization, CAS/cursor semantics, public error vocabulary and runtime passivity.

CONTRACT_AND_SEMANTIC_REVIEW:
private_snapshot_bounded: yes
admin_summary_provider_identifier_free: yes
matching_gdrive_adapter_private_read: yes
admin_read_only_sanitized_access: yes
matching_gdrive_adapter_commit_only: yes
mandatory_idempotency_key: yes
expected_state_version_present: yes
expected_cursor_generation_present: yes
cursor_advance_exactly_contiguous: yes
cursor_overflow_fails_closed: yes
storage_i64_numeric_boundary_checked: yes
canonical_vault_paths_checked: yes
mapping_echo_state_shape_checked: yes
provider_identifier_and_text_bounds_present: yes
commit_outcome_vocabulary_stable: yes
public_error_categories_safe: yes
Storage_or_Server_transaction_execution_in_API: no

FIXTURE_REVIEW:
fixture_schema_version: 1
strict_root_shape: yes
private_snapshot_roundtrip: yes
admin_summary_roundtrip: yes
commit_request_roundtrip: yes
closed_echo_operation_outcome_error_vocabularies: yes
synthetic_values_only: yes
raw_cursor_in_fixture: no
real_credentials_or_provider_payloads: no
note: Synthetic provider identifiers/facts in the private compatibility fixture are acceptable as deterministic wire examples; the blocker concerns their recursive Debug exposure at runtime, not the existence of synthetic fixture values.

PASSIVITY_AND_SCOPE:
Axum_or_route_registration: none
Server_execution: none
Storage_or_SQLx_calls: none
Core_policy: none
provider_or_OAuth_calls: none
scheduler_or_background_tasks: none
status_or_operator_control_contracts: none
sibling_component_changes_during_review: none
product_changes_during_review: none
workflow_changes_during_review: none

CI:
workflow: Component CI
run_id: 29440533856
run_number: 2017
attempt: 1
head_sha: 77945118a37c6e8efec04ecd054e0e5a5e4435ba
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
overall_conclusion: success
ci_status: CI_GREEN
report_commit_ci_skip_used: yes
report_commit_ci_skip_reason: review report only after authoritative exact-SHA CI; not used as CI evidence

OTHER_FINDINGS:
authorization_defects: none
CAS_or_cursor_semantic_defects: none
public_admin_output_leaks: none found
public_error_leaks: none found
fixture_real_secret_leaks: none
protected_scope_violations: none

BLOCKERS:
- Fix API-GDA-P1-DEBUG-PROVIDER-FACT-LEAK and obtain a new full exact-SHA green Component CI run.

NEXT_RECOMMENDED_AGENT:
fixer-worker for one focused private Debug/redaction correction

DOWNSTREAM_AUTHORIZATION:
Server GDrive application/transaction phase: not authorized until focused fix and subsequent clean acceptance

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Exact candidate SHA 77945118a37c6e8efec04ecd054e0e5a5e4435ba is functionally coherent, passive and CI-green, but derived Debug on private mapping/snapshot/commit DTOs and recursive authenticated-request formatting expose provider/path facts beyond the assigned secrecy boundary. A minimal API-only redaction fix plus exact-SHA green CI and clean re-review is required. No repository merge-readiness claim is made.

PUSHED:
yes
