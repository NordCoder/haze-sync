# Implementation Log: storage

## Entries

### 2026-07-10 — W1/STOR-P9 storage test-support and integration harness hardening

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Hardened Storage test support without adding production runtime dependencies. PostgreSQL integration helpers now require the dedicated `HAZE_SYNC_TEST_DATABASE_URL`, ignore general application `DATABASE_URL`, reject missing, malformed, non-test, and production-looking database configuration through redacted errors, and expose a strict prepare helper. Schema setup is serialized with a transaction-scoped PostgreSQL advisory lock, applies migrations only to an empty schema, accepts an already-complete initial schema, and rejects partial schemas rather than guessing. Default crate tests compile the PostgreSQL helper implementation without requiring a live database, while feature-gated repository roundtrips require explicit configuration and fail on connection/setup errors instead of silently passing. Added bounded stable fixture namespaces, explicit temporary object-root cleanup, focused unit tests, and a test-support runbook with exact commands and isolation rules.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Use Component CI for workspace fmt/check/test/clippy verification. Run `cargo test -p haze-sync-storage --features test-support` with a dedicated reachable PostgreSQL test database, then perform the mandatory clean-code review.

---

### 2026-07-10 — W1/STOR-P8 adapter mapping and worktree state repository support

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Added passive repository support for the existing `gdrive_mapping` and `worktree_state` tables without moving adapter semantics into Storage. Google Drive mapping helpers now upsert complete caller-decided fact sets by normalized vault path and read mappings by path or opaque Drive file id. Worktree helpers upsert and read complete materialization-state fact sets. Repository inputs use shared validated path, revision, and SHA-256 types; persisted rows are revalidated for canonical paths, identifiers, hashes, non-negative Core sequences, provider metadata shape, and typed UTC timestamps before crossing the Storage boundary. Added safe repository error variants plus focused unit and feature-gated PostgreSQL transaction roundtrips. Provider calls, identity interpretation, echo handling, delete-candidate decisions, import/export direction, filesystem scans, materialization, and dirty-state semantics remain outside Storage.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Use Component CI for workspace fmt/check/test/clippy verification. Run the feature-gated PostgreSQL tests with an explicit safe test database URL, then perform the mandatory clean-code review.

---

### 2026-07-10 — W1/STOR-P7 idempotency and cursor repository support

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Hardened durable idempotency and adapter cursor persistence without absorbing Core or adapter behavior. Idempotency helpers now document the accepted Core stored-response snapshot boundary, validate persisted adapter/key/hash metadata safely, preserve the first writer's response for same-request replay and different-request conflict outcomes, and include unit plus feature-gated PostgreSQL roundtrips. Cursor initialization and monotonic updates now use single atomic upserts that handle missing rows and concurrent initializers without same-statement snapshot gaps; regression requests preserve persisted sequence, external cursor, success timestamp, and updated timestamp. Added a serializable cursor summary that exposes only cursor presence, never raw external cursor JSON, plus unit and feature-gated PostgreSQL coverage.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Use Component CI for workspace fmt/check/test/clippy verification. Run the feature-gated PostgreSQL tests in an environment with an explicit safe test database URL, then perform the mandatory clean-code review.

---

### 2026-07-10 — W1/STOR-P6C conflict and tombstone clean-code review

Agent:
Clean-Code Reviewer

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the clean-code commits.

Summary:
Reviewed STOR-P6 and its CI compatibility fix across conflict lifecycle updates, bounded/source-compatible listing, tombstone restore metadata, operation-log mapping, caller-owned transaction composition, and non-goal preservation. Hardened all operation-log read paths so unsupported persisted operation kinds return the safe `InvalidOperationKind` repository error instead of escaping as unvalidated strings. Split operation-log unit and feature-gated PostgreSQL tests into dedicated submodules to keep production repository code focused without weakening coverage or changing public APIs.

Status:
CLEAN_ACCEPT_PENDING_CI

Follow-ups:
Use Component CI for fmt/check/test/clippy verification of the clean-code source/docs head. No additional STOR-P6 behavior is deferred.

---

### 2026-07-10 — W1/STOR-P6 conflict, tombstone, and delete repository support

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Added passive conflict insertion, bounded status listing, lookup, safe persisted-status validation, and guarded resolve/ignore metadata updates. Added one-shot tombstone restore metadata updates while preserving retention and deleted-revision facts. Extended operation-log changes-feed mapping to reject invalid negative persisted sizes safely, added conflict/delete/restore event metadata tests, and expanded the feature-gated PostgreSQL flow test to cover conflict, tombstone, lifecycle-update, and changes-feed roundtrips inside one caller-owned transaction. Documented that full `accept_conflict` content replacement and restore orchestration remain Core/API/Server fan-in responsibilities.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run cargo fmt/check/test/clippy through CI or an environment with shell execution. Next storage phase should proceed through clean-code review before final lifecycle acceptance.

---

### 2026-07-09 — W1/STOR-P5 normal file flow repository support

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Verified the normal file flow repository surface while keeping Storage passive and policy-free. Added content blob metadata input tests, sync object/current-revision input tests, operation-log append metadata tests, changes-page sentinel behavior tests, and a feature-gated PostgreSQL roundtrip covering caller-owned transaction composition with path advisory lock, content blob create/read, sync object create/read/current-revision update, immutable file revision insert/read/current lookup, operation-log append/read, and changes-page output. Documented that normal file flow is caller-composed repository work under Server/Core transaction orchestration.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run cargo fmt/check/test/clippy through CI or an environment with shell execution. Next storage phase should proceed through clean-code review before final CI merge readiness.

---

### 2026-07-09 — W1/STOR-P4 repository validation and safe error boundary hardening

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See component/storage branch history for the implementation commits.

Summary:
Audited repository helpers for caller-owned executor/transaction boundaries and strengthened storage-level validation coverage. Revision list helpers now use the shared repository limit validator before querying. Repository boundary tests cover list-limit bounds, non-negative sequence validation, representable `size_bytes` conversion, stable safe error codes/messages/display, and mapping raw SQLx errors to a path-free/secret-free repository error. Documented transaction-sensitive repository helper groups and preserved Storage/Core/API/Server boundaries.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run cargo fmt/check/test/clippy through CI or an environment with shell execution. Next storage phase should proceed through clean-code review before final CI merge readiness.

---

### 2026-07-09 — W1/STOR-P3 object-store hardening

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See branch history for the implementation commits.

Summary:
Hardened object-store verification coverage while preserving the existing content-addressed blob layout and runtime behavior. Added tests for missing blob path-free errors, corrupted committed blob verification through read/exists/stat, unexpected directory entries at blob paths, temporary blob cleanup after failed commit, duplicate writes, hash mismatch handling, and path-free error Display output. Documented that the local object-store root is caller-owned durable storage while Storage owns only the internal hash-addressed layout below that root.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run cargo fmt/check/test/clippy through CI or an environment with shell execution. Next storage phase should proceed through clean-code review before CI merge readiness.

---

### 2026-07-06 — W1/STOR-P2 schema and row-model audit

Agent:
Implementation Worker

Branch:
component/storage

Prompt:
crates/haze-sync-storage/control/prompt.md

Report:
crates/haze-sync-storage/control/report.md

Commit(s):
See branch history for the implementation commits.

Summary:
Audited schema table-name metadata, initial migration filename ordering, and passive row models against migrations 0001 through 0009. Added schema tests that bind INITIAL_MIGRATIONS to the actual migration files and verify table metadata coverage. Added explicit sensitive/internal-only row-field metadata for token hashes, object-store path metadata, adapter cursors, idempotency material, provider identifiers, and audit metadata. Added row serialization roundtrip tests for representative rows with sensitive fields, optional timestamps, JSON cursor fields, and JSON audit metadata. No migration schema changes were required.

Status:
SELF_ACCEPT_PENDING_CI

Follow-ups:
Run cargo fmt/check/test/clippy through CI or an environment with shell execution. Next storage phase should proceed through clean-code review before CI merge readiness.

---

### 2026-07-05 — STOR-P1 component contract and planning normalization

Agent:
Architect

Branch:
component/storage

Prompt:
User requested continuing component documentation and implementation-plan writing after API.

Report:
Conversation summary; no component control report was written because this is an Architect documentation/planning pass, not an implementation-worker execution.

Commit(s):
See branch history after this documentation pass.

Summary:
Replaced scaffold-level Storage component docs with a real component contract, dependency map, phased implementation plan, component decisions, and this planning baseline log entry. The pass documented Storage as the durable persistence component owning schema metadata, row models, repository helpers, object-store primitives, transaction-scoped path locks, and gated test support while preserving Core/API/Server/adapter boundaries.

Status:
ARCHITECT_ACCEPT_PENDING_REVIEW

Follow-ups:
Execute STOR-P2 through STOR-P9 through normal implementation -> clean-code -> CI -> fixer lifecycle when scheduled. Keep Storage policy-free and avoid direct API/Server/adapter/provider dependencies.

---

Use this format for future entries:

~~~text
### YYYY-MM-DD — <wave>/<phase>

Agent:
Branch:
Prompt:
Report:
Commit(s):
Summary:
Status:
Follow-ups:
~~~