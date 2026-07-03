# W3 Core Safety Test Strategy

This directory documents the Wave 3 Core safety coverage originally added by `W3-P8 — Core safety tests` and converted by `W3-F1E — Safety regression fan-in` after the W3-F1A-D fan-in slices were merged.

## What runs by default

Default W3 safety coverage is fake-backed, DTO-only, or dependency-free route-contract coverage. It does not require PostgreSQL, real providers, network access, runtime services, secrets, Google Drive OAuth, adapter loops, or production object-store roots.

The default coverage now asserts:

- stale-base writes with different existing content produce `conflict_saved` metadata and do not overwrite current content;
- unknown-base writes with different existing content produce `conflict_saved` metadata and do not overwrite current content;
- explicit-null-base writes against an existing different file produce `conflict_saved` metadata;
- stale, unknown, and explicit-null bases with the same content are ignored as `same_content`;
- conflict materialized paths stay under `_haze_conflicts/open`;
- source paths already under `_haze_conflicts` are rejected to avoid recursive conflict explosions;
- GET conflict listing maps open conflicts to safe public DTOs, or safely returns an empty dependency-free response without storage;
- conflict resolution request actions parse safely, while route/runtime behavior remains narrow where full product behavior is not implemented;
- DELETE route helpers require `Idempotency-Key` and keep delete errors safe;
- tombstone metadata can be created without hard deleting content or revision history;
- delete guard blocks unsafe batches by count and ratio, and requires an explicit scoped unlock where configured;
- idempotent delete replays the same request and rejects a different request for the same key;
- W3 operation kinds such as `delete_file` and `conflict_resolved` are represented in changes-feed DTOs;
- admin/status and doctor public outputs remain secret-free in their crate-local tests;
- W2 PUT, GET, changes, idempotency, and operation-log semantics remain covered.

## Still deferred or spec-only behavior

Some product behavior is intentionally not implemented by W3-F1E and remains deferred to later phases or opt-in DB-backed coverage:

- full `accept_conflict` resolution that replaces the main file by creating a new current revision;
- adapter/provider runtime behavior;
- hard delete, cleanup workers, retention jobs, restore workflows, or background jobs;
- broad E2E database scenarios unless they use an existing explicit opt-in safe harness.

## Safety notes

The tests should remain safe for default CI/local runs:

1. keep default tests pure, fake-backed, DTO-only, or dependency-free;
2. keep real database tests ignored or behind existing explicit opt-in harnesses;
3. do not require provider credentials, network access, production paths, or secrets;
4. do not serialize raw file content, tokens, token hashes, database URLs, local absolute paths, request bodies, stack traces, provider payloads, SDK/client internals, or runtime internals in public outputs.
