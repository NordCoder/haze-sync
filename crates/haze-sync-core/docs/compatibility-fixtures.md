# Core Compatibility Fixtures

## Purpose

The V1 compatibility catalog at
`crates/haze-sync-core/fixtures/compatibility/v1/core-compatibility.json`
contains deterministic, language-neutral examples of public Core values.
Downstream API, Server, Storage, CLI, and adapter components may use these
examples to verify semantic mapping without importing private Core internals or
reimplementing Core policy.

The catalog is test data, not a new runtime DTO. Its top-level `fixture_set`,
`version`, `examples`, and example labels organize the fixtures only. They must
not be copied into HTTP responses or persistence rows unless another component
explicitly owns that mapping.

## Included examples

The V1 catalog covers:

- an accepted normal revision write;
- an ignored same-content write;
- a stale-base write whose incoming metadata is preserved as `conflict_saved`;
- a content hash mismatch;
- tombstone metadata with retention and no restore state;
- a default-policy delete-guard block requiring manual unlock;
- idempotency replay for the same request;
- idempotency conflict for a different request fingerprint;
- advanced, unchanged, and rejected-regression operation-log cursor outcomes;
- a passive doctor report with an intentionally skipped live DB check;
- a passive doctor report containing explicit `not_run` and `placeholder` states.

All identifiers, paths, timestamps, hashes, counts, and JSON bodies are synthetic.
The catalog contains no raw file bytes, idempotency keys, credentials, token
hashes, database URLs, provider payloads, or local absolute paths.

## Stable semantic contract

For the public Core models represented in the catalog, downstream mappings may
rely on:

- JSON field names emitted by the represented public Core type;
- tagged status/outcome vocabulary such as `accepted_new_revision`,
  `ignored_duplicate_same_content`, `rejected_hash_mismatch`,
  `blocked_requires_manual_unlock`, `replay_same_request`,
  `conflict_different_request`, `advanced`, `unchanged`, and
  `rejected_regression`;
- canonical identifier, vault-path, timestamp, and `sha256:<hex>` formats;
- accepted revision metadata and operation metadata being separate fields;
- same-content outcomes carrying the unchanged current revision;
- hash-mismatch outcomes exposing only expected and actual hashes;
- conflict-preservation serialization omitting incoming raw bytes while retaining
  path, adapter, hash, size, current revision, provided base, and policy hint;
- tombstones carrying delete intent, revision references, retention metadata, and
  restore metadata without implying physical deletion;
- delete-guard decisions remaining pure classifications rather than execution;
- idempotency replay snapshots containing only a final status, safe normalized
  replay headers, and a public JSON body;
- idempotency conflicts carrying fingerprints rather than raw request bodies or
  idempotency keys;
- cursor updates being monotonic classifications only;
- doctor summary counts matching the ordered checks and preserving distinct
  `skipped`, `not_run`, and `placeholder` semantics.

The `conflict_saved` example deserves special attention. The current Rust enum
retains the serialized tag `rejected_stale_or_unknown_base` for compatibility,
while `UpsertOutcome::public_status()` returns `conflict_saved` when the nested
`conflict_saved` plan is present. Downstream public API mapping must use the
semantic result, not expose the Rust variant name as an HTTP policy decision.

## Internal implementation details

The fixtures do not make the following details stable cross-component contracts:

- Rust module paths, enum variant names, helper functions, field visibility, or
  accessor choices;
- repository/content-store/operation-log traits and transaction boundaries;
- in-memory incoming conflict bytes, which are deliberately skipped by serde;
- concrete database rows, indexes, locks, migrations, or object-store layout;
- HTTP route shapes, status codes, headers, request IDs, or API-owned DTO names;
- provider file IDs, page tokens, webhook payloads, worktree paths, or plugin
  state;
- live doctor probing or repair behavior;
- textual JSON whitespace or object order as an external protocol requirement.

The repository test keeps one canonical pretty-printed representation so diffs
are reviewable. Consumers should map by field names and semantics, not by byte
offsets or textual order.

## Downstream mapping guidance

- API/Server may map `accepted_write` to an API-owned accepted response, but the
  fixture does not define that HTTP response.
- API/Server may map `same_content` to an ignored response with reason
  `same_content`.
- API/Server and Storage must treat `conflict_saved` as a preserve-current plan;
  conflict IDs, materialized paths, persistence rows, and operation append results
  remain downstream-owned.
- Storage may persist tombstone metadata and idempotency snapshots, but the
  fixture does not define schemas or transaction boundaries.
- Adapters may use cursor outcomes to decide whether a durable cursor update is
  allowed, but Core does not perform that update.
- CLI/Server may render doctor reports, but skipped or unavailable checks must not
  be presented as successful live checks.
- No consumer should deserialize or expose `StoredIdempotencyRecord` as public
  output because that storage model contains the raw validated idempotency key.

## Stability test

`crates/haze-sync-core/tests/compatibility_fixtures.rs` enforces the catalog by:

1. deserializing every example into its current public Core type;
2. requiring canonical round-trip serialization to match the committed catalog;
3. recomputing revision, delete-guard, idempotency, cursor, tombstone, and doctor
   decisions through public Core APIs;
4. checking semantic statuses and invariant-sensitive fields;
5. scanning the catalog for secret-bearing, raw-byte, and unsafe-path markers.

A change that intentionally alters represented public fields or semantics should
update the catalog, its tests, this document, and the fixture `version` after
reviewing affected downstream mappings. Internal refactors that preserve the
public values should not require fixture changes.
