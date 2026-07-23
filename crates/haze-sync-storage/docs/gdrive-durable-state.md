# Google Drive durable state

## Ownership

Storage owns the passive PostgreSQL schema and repository boundary for restart-safe Google Drive facts. Server will later own transaction choreography and operation timing. The standalone Google Drive Adapter must not receive a database URL or use SQLx directly. Core remains authoritative for revision, conflict, deletion, tombstone, idempotency, and unlock policy.

## Migration 0011

`0011_gdrive_durable_state.sql` adds three adapter-scoped tables without changing accepted migrations or deleting historical `gdrive_mapping` rows:

- `gdrive_adapter_state`: state format/version, opaque Drive cursor generation, Core export checkpoint, and last successful operation references;
- `gdrive_durable_items`: Drive/Core mapping, provider reconciliation facts, echo confirmation, and delete-candidate observations keyed by `(adapter_id, path)`;
- `gdrive_operations`: retry-safe typed operation fingerprints and committed outcomes keyed by `(adapter_id, operation_id)`.

The migration fails before creation if any new table name already exists, using the fixed error `gdrive durable state structures require explicit operator review`. Existing `gdrive_mapping` data remains intact and is not assigned to a new adapter identity automatically.

## Compare-and-commit

`repositories::gdrive_state` exposes passive helpers over caller-owned PostgreSQL transactions:

1. initialize one version-1 aggregate for a registered adapter;
2. load a bounded, path-ordered adapter snapshot;
3. lock the aggregate and require an exact expected `state_version`;
4. validate exact cursor generation progression and non-regressing Core export checkpoint;
5. persist one caller-confirmed mapping/echo/delete-candidate fact set;
6. advance the aggregate with compare-and-set semantics;
7. store a deterministic operation outcome for replay.

A duplicate operation ID with the same kind and facts fingerprint replays its stored outcome. The same identity with different facts fails safely. Any validation error, stale expected version, database error, or caller rollback leaves all state unchanged. Storage does not start or commit a hidden transaction.

## Secrecy

The schema stores no OAuth credentials, bearer tokens, token hashes, request bodies, provider payloads, file bytes, or public status vocabulary. Opaque cursors and operation fingerprints are persisted only for correctness and are redacted from `Debug`, `Display`, errors, tests, and reports. Provider identifiers remain internal Storage facts and require Server/API sanitization before public rendering.

## Downstream fan-in

Server/API fan-in may consume the accepted Storage repository only through caller-owned transactions and sanitized service models. It must not issue ad hoc SQL or let the Adapter access these tables directly. Provider calls, scheduling, retries, import/export decisions, delete confirmation policy, admin unlock, and public DTOs remain outside Storage.
