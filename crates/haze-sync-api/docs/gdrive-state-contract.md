# Passive Google Drive Durable-State Contract

Phase: `API-GDA-P1-CONTRACTS`

Accepted inputs:

- API baseline `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- Storage durable-state contract `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`;
- GDrive fan-in architecture acceptance represented by blob `14c427880e1201d851cdc9ee04b9cd0e83334de4`.

## Ownership

API owns passive JSON shapes, pure route parsing, authenticated identity/role checks, bounded validation, safe error vocabulary and compatibility fixtures for:

```text
GET  /v1/adapters/{adapter_id}/gdrive/state
POST /v1/adapters/{adapter_id}/gdrive/state/commit
```

Server owns token lookup, HTTP registration, application choreography and caller-owned database transactions. Storage owns durable persistence and compare-and-commit enforcement. The standalone GDrive Adapter owns provider/OAuth/scheduling behavior and never receives database access.

## GET state

A matching authenticated `gdrive_adapter` principal receives the bounded private adapter snapshot. An authenticated admin receives only the sanitized aggregate summary.

The private response contains:

- adapter identity and state format/version;
- cursor generation and presence, never the raw cursor;
- Core export checkpoint;
- last import/export/provider-mutation operation identifiers;
- at most 500 path-ordered mapping, echo and delete-candidate fact summaries;
- an optional path cursor for bounded pagination.

Provider identifiers are bounded opaque atoms used only by the matching authenticated adapter response. Their DTO formatting is redacted. Admin output contains counts and presence flags instead of provider identifiers, mapping paths or operation identifiers.

## POST compare-and-commit

The private commit contract requires:

- a matching authenticated `gdrive_adapter` principal;
- mandatory `Idempotency-Key` header metadata;
- expected aggregate state version;
- expected cursor generation;
- zero or one raw cursor transition advancing exactly one generation;
- optional non-negative Core export checkpoint;
- optional typed mapping/echo/delete-candidate facts;
- mandatory operation id, kind and SHA-256 facts fingerprint.

Raw cursor values exist only in the authenticated commit body. They are serialized for Server handoff but redacted from `Debug`, `Display`, public errors and compatibility fixtures.

API validates shape, bounds, identity, exact cursor increment and internal DTO consistency. It does not inspect current durable state, decide whether a checkpoint regresses, execute a transaction or translate provider policy. Server and Storage enforce current-state CAS, cursor/checkpoint invariants, atomicity and replay.

Stable submission outcomes are:

```text
committed
replayed
stale_state
cursor_regression
cursor_gap
mapping_conflict
idempotency_conflict
validation_failed
```

Stable safe error codes are:

```text
unauthorized
forbidden
adapter_not_found
state_version_mismatch
invalid_cursor_state
stale_state
cursor_regression
cursor_gap
mapping_conflict
idempotency_conflict
validation_error
unavailable
internal
```

No error contains raw cursor, facts fingerprint, idempotency key, provider identifier, OAuth value, token hash, provider payload, database error or local path.

## Compatibility evidence

The deterministic fixture is:

```text
crates/haze-sync-api/fixtures/gdrive-state-contract-v1.json
```

The strict verifier is:

```text
crates/haze-sync-api/tests/gdrive_state_compatibility_fixture.rs
```

The fixture covers the private bounded snapshot, admin sanitization, strict-empty cursor transition omission, mandatory operation facts, all commit outcomes and complete closed vocabularies. A separate unit/integration test proves that a real private cursor transition serializes only for authenticated commit handoff and remains absent from formatting and the fixture.

## Non-goals

This phase adds no Axum registration, Server state, SQLx/Storage calls, Core policy, provider/OAuth client, scheduler, status ingestion, operator controls, background tasks, retries, filesystem behavior, sibling-component changes or workflow changes.
