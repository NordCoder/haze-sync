REPORT_TYPE:
CLEAN_CODE_REVIEW

STATUS:
CLEAN_NEEDS_FIX

AGENT:
role: clean-code-reviewer
agent_execution_id: server-srv-gda-p1-clean-functional-review-20260716-b0ae522
chat_name: server — W1 SRV-GDA-P1 Clean Functional Review

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: SRV-GDA-P1-CLEAN-FUNCTIONAL-REVIEW

CANDIDATE:
code_bearing_sha: b0ae522229bbc6422763a2cd075b995768346963
initial_implementation_report_blob: ad664f9a0dce10c6e49fffedec9a40b74366816c
completion_report_blob: efc05e8e2a903e7197b90cc996d471f753f5a7fa
fixer_report_blob: 7df311e91b8a4e7023b02568304cdeb77b3e9c2e

SUMMARY:
Route registration, authentication, adapter identity/type resolution, caller-owned PostgreSQL transaction ownership, rollback atomicity, accepted owner blob identity, real PostgreSQL route evidence, concurrency/replay/isolation and secrecy are sound. Two substantive Server outcome-classification defects remain in POST compare-and-commit: persisted cursor-generation mismatch is exposed as cursor_gap, and internal/unexpected Storage failures are exposed as validation_failed. Both misclassify accepted API semantics and can direct the downstream client toward the wrong recovery behavior.

POSITIVE_FINDINGS:
- GET /v1/adapters/{adapter_id}/gdrive/state and POST /v1/adapters/{adapter_id}/gdrive/state/commit are registered under the existing /v1 router.
- API parsing requires authentication, matching gdrive_adapter identity for private read/commit, Admin-only sanitized read, bounded query/body values and mandatory syntactically valid Idempotency-Key metadata.
- Server verifies an enabled sync_adapters row with role gdrive_adapter before any GDrive Storage repository access.
- GET opens one caller-owned transaction, uses the accepted bounded snapshot repository, rolls back every failed path and commits a validated private snapshot before response mapping.
- POST opens one caller-owned transaction, invokes accepted compare-and-commit once, commits Committed/Replayed and rolls back every repository error.
- Storage exact state-version CAS, row locking, contiguous cursor advancement, checkpoint non-regression, mapping/echo/delete-candidate/operation atomicity, deterministic replay and adapter isolation remain unchanged.
- Accepted operation tickets/facts and private cursor/provider values remain redacted from Debug/public errors.
- No provider/OAuth/Core policy/scheduler/status-control/CLI/Deployment/workflow or sibling-branch expansion occurred.

AUTHORIZATION_REVIEW:
- missing authentication -> 401 unauthorized
- unrelated role -> 403 forbidden
- mismatched GDrive adapter identity -> 403 forbidden
- matching principal against registered non-GDrive row -> 403 before durable-state access
- matching enabled GDrive adapter -> private bounded state and commit access
- Admin -> sanitized GET only; commit remains forbidden
result: clean

TRANSACTION_AND_ATOMICITY_REVIEW:
- read and write repositories receive caller-owned Transaction<Postgres>
- failed adapter verification, missing state, snapshot conversion/validation and repository reads roll back
- Committed and Replayed outcomes commit
- every compare-and-commit error rolls back before response construction
- item upsert, aggregate CAS/update and operation insertion remain one transaction
- concurrent expected-version-zero writers yield one committed winner and one stale loser
- duplicate provider mapping failure is proven to leave state version, cursor/checkpoint, mapping, echo/delete-candidate and operation facts unchanged
result: clean

SUBSTANTIVE_FINDING_1:
id: SRV-GDA-P1-CURSOR-MISMATCH-MISCLASSIFIED
location: crates/haze-sync-server/src/routes/gdrive.rs::commit_error_response
current_behavior:
- RepositoryError::GDriveCursorGenerationMismatch maps to HTTP 409 GDriveStateCommitResponse::CursorGap.
problem:
- Storage uses GDriveCursorGenerationMismatch when expected_generation does not equal the locked persisted generation.
- CursorGap is reserved for a requested next_generation that skips the exact contiguous successor.
- The accepted API owns GDriveStateRouteError::InvalidCursorState / error code invalid_cursor_state for persisted cursor-state mismatch.
impact:
- downstream client can interpret a stale/inconsistent persisted-generation expectation as a malformed non-contiguous transition and apply the wrong recovery path instead of reloading state.
required_fix:
- map GDriveCursorGenerationMismatch to a fixed 409 GDriveStateErrorResponse with code invalid_cursor_state, not the cursor_gap commit outcome.
- add a real PostgreSQL route test that uses a current state_version with a mismatched expected cursor generation and verifies rollback plus invalid_cursor_state.

SUBSTANTIVE_FINDING_2:
id: SRV-GDA-P1-INTERNAL-AS-VALIDATION
location: crates/haze-sync-server/src/routes/gdrive.rs::commit_error_response
current_behavior:
- every unmatched RepositoryError, including DatabaseOperationFailed, UnsupportedGDriveStateVersion and StateVersionOverflow, maps to HTTP 500 GDriveStateCommitResponse::ValidationFailed.
problem:
- validation_failed is an accepted caller/input outcome; the accepted API separately defines GDriveStateErrorCode::Internal and a fixed internal error envelope.
- the existing rollback PostgreSQL test explicitly receives a DatabaseOperationFailed path but asserts status=validation_failed, preserving the misclassification.
impact:
- downstream client cannot distinguish invalid submitted facts from an internal/database failure and may fail closed permanently instead of retrying/reporting a transient or operator-visible internal condition.
required_fix:
- split repository mapping into typed commit outcomes versus safe route errors.
- map DatabaseOperationFailed and other internal/unexpected invariant failures to GDriveHttpError::internal() / {"error":{"code":"internal",...}} after rollback.
- retain ValidationFailed only for request/storage validation categories.
- update the rollback test to assert HTTP 500 internal error envelope while preserving all no-partial-facts and secrecy assertions.

IDEMPOTENCY_REVIEW:
- mandatory Idempotency-Key is parsed, validated and redacted.
- durable replay/conflict is owned by accepted Storage operation_id plus facts fingerprint and typed persisted facts.
- no raw Idempotency-Key is persisted or exposed.
- no owner-contract mismatch is declared in this review; the accepted Storage schema has no Idempotency-Key field.
result: no blocking finding in this phase

POSTGRESQL_ROUTE_EVIDENCE:
- private and Admin reads through assembled Axum router
- missing auth, unrelated role, identity mismatch and registered type mismatch
- committed and replayed
- stale expected state
- cursor regression and gap
- mapping conflict and operation/fingerprint idempotency conflict
- database constraint failure with rollback/no partial facts and safe redaction
- concurrent writer stale loser
- second-adapter isolation
coverage_gap:
- no test for locked persisted cursor-generation mismatch classification
- internal DB failure test expects the wrong validation_failed body category

DEPENDENCY_IDENTITY:
API accepted SHA: c60c3976696da1970d539e5cff6e9f74a61fc10e
- contracts/errors.rs: 0f59990c9a248e6e2a312419b89d73ae1cacc507 exact
- contracts/headers.rs: b146f269056a6ff20cca61591c19cdc14b9085b6 exact
- dto/gdrive.rs: 8098457eee373f63210fbd381c6487a054d7609f exact
- dto/mod.rs: c4f992c1b21c555dd07c3a2e4253594ea1a715ba exact
- routes/gdrive.rs: ef7f8dc1d02b3742e76bb99c66ee39703bee3b4b exact
- routes/mod.rs: 2f328be883a2e71afed414da41685831adfb0009 exact
Storage accepted SHA: 3617bd1cf947fdd394f1ab29d4b992f7b8859a84
- repositories/gdrive_state.rs: 4ec6e3afa096fdbfb291c847a144d5082f9c1adf exact
- repositories/gdrive_state/types.rs: 1f0d5ac29bbe52d5fc7579b9a1120179363cdfb6 exact
- repositories/gdrive_state/repository.rs: 1b9f44d6b4672fff872347cd633932e836a76ef8 exact
- repositories/gdrive_state/validation.rs: a8c7eb51f3706ffb65d95070bd55c3830cf2b530 exact
- repositories/gdrive_state/tests.rs: 6530d03703313138cb040c5ea90a2055cb10f694 exact
- repositories/gdrive_state/postgres_contract_tests.rs: 26b60d2d6cd761e6f075130f2c75f8a3bf07039f exact
- repositories/mod.rs: f490bbfb3699950216ba9afac8e23a1828cf515f exact
- schema/mod.rs: b4d47719ed9c2ee2d5ee7f1c46ce34dd32f7fbcb exact
- migrations/0011_gdrive_durable_state.sql: befea883357637171ea32928412916f944d977db exact
- test_support/postgres.rs: f763cff8c3f1058c7320c61d4678663f613153f8 exact
- test_support/postgres/implementation.rs: 01c52d6aed7bbb6b52cefc08fb55eb54bb29ba1e exact
- test_support/postgres/tests.rs: 1226e04b22fbf4714fe0e505642e3b2521939c08 exact
accepted_owner_semantics_modified: no

SECRECY_REVIEW:
- raw cursor absent from GET private/Admin response and all public errors
- Admin response excludes mappings, paths, provider IDs and operation IDs
- API private DTO and route Debug implementations are marker-only/redacted
- SQLx errors collapse to stable RepositoryError and no database URL/SQL text is returned
- rollback test excludes cursor, Idempotency-Key, provider ID, timestamps, postgres URL and sqlx markers
result: clean except semantic error category findings above

CI:
workflow: Component CI
run_id: 29490745222
run_number: 2041
run_attempt: 1
head_sha: b0ae522229bbc6422763a2cd075b995768346963
conclusion: success
db_capable: yes
cargo_fmt: success
cargo_check: success
cargo_test: success
cargo_clippy: success
diagnostics_finalizer: success
diagnostics_upload: skipped_expected_no_failure
assessment: green CI does not cover cursor-generation mismatch and currently asserts the incorrect internal-failure category

LATER_COMMIT_VALIDATION:
Compared b0ae522229bbc6422763a2cd075b995768346963 with pre-report PR head b32ae035178a7ba029c1809e2d83236aa78e496d. Later changes are confined to Server control prompt/state/log history. No later product or tooling commit corrected or invalidated the candidate.

CODE_CHANGES_DURING_REVIEW:
none

BLOCKERS:
- persisted cursor-generation mismatch is publicly misclassified as cursor_gap
- internal/unexpected Storage failures are publicly misclassified as validation_failed

NEXT_RECOMMENDED_AGENT:
fixer-worker

NEXT_GATE:
Apply the two focused Server-only outcome-mapping fixes and PostgreSQL assertions, obtain full exact-SHA DB-capable green CI, then repeat focused clean review. Do not start GDA-GDA-P2 HTTP/durable-state client work yet.

FINAL_VERDICT:
CLEAN_NEEDS_FIX. Authorization, route registration, transaction ownership, rollback atomicity, accepted owner identity, PostgreSQL evidence, replay/concurrency/isolation and secrecy are accepted. Server outcome classification remains incorrect for persisted cursor-generation mismatch and internal/unexpected Storage failures, so the GDrive HTTP/durable-state client phase must remain blocked pending a focused fix and re-review.

PUSHED:
yes
