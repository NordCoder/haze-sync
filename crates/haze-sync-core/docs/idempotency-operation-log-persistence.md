# Idempotency and Operation-Log Durable Fan-In

## Scope

This document defines how Storage and Server should persist and consume the pure
Core primitives in `idempotency` and `operation_log`. Core does not open a
database, append durable operations, implement HTTP middleware, or update adapter
cursors.

## Idempotency record

Durable lookup identity is the exact pair:

```text
(adapter_id, idempotency_key)
```

Storage must persist, at minimum:

- the authenticated adapter scope;
- the raw validated idempotency key;
- the deterministic request fingerprint;
- the validated stored response snapshot;
- lifecycle timestamps required by the Storage retention policy.

The raw key is persistence-only data. `IdempotencyKey` formatting is redacted,
but `as_str`, `into_string`, and serialized `StoredIdempotencyRecord` values still
contain the raw key for exact durable lookup. API responses, logs, reports,
metrics labels, traces, and error details must never expose it.

## Fingerprint input

Server must construct the fingerprint from canonical safe request metadata. For a
file write this normally includes the semantic method/action, normalized vault
path, explicit base revision value, expected content hash, and any other field
that changes the write result.

Fingerprint metadata must exclude:

- authorization or provider credentials;
- the idempotency key itself;
- raw request/file bytes;
- cookies and session material;
- database URLs, local absolute paths, and internal error details;
- volatile transport metadata that does not change request semantics.

Core preserves JSON object-key independence recursively. Array order and scalar
values remain significant. Changing the fingerprint algorithm or the selected
semantic fields is a cross-component compatibility change.

## Atomic write/replay flow

Server and Storage should implement the following transaction boundary:

1. Authenticate the adapter and validate the idempotency key.
2. Compute the request fingerprint from safe semantic metadata.
3. Load the record by exact adapter scope and raw key.
4. If a record exists, use `IdempotencyService::evaluate`:
   - same fingerprint: replay the stored response;
   - different fingerprint: return an idempotency conflict without executing the write.
5. If no record exists, execute the domain write and persist its idempotency record
   atomically with the durable result whenever the storage design permits it.
6. Resolve concurrent insert races by reloading the winning record and performing
   the same fingerprint comparison.

Core assumes the optional record passed to `evaluate` was already loaded by the
exact scope/key pair. It does not verify lookup identity itself.

## Stored response limitations

`StoredIdempotencyResponse` contains only:

- an HTTP status code;
- validated replay headers with normalized lowercase names;
- a public JSON body.

It is not a general HTTP response archive. It must not contain raw file bodies,
streaming responses, authorization headers, cookies, API/auth tokens,
idempotency-key headers, provider payloads, stack traces, or raw database errors.
Deserialization re-runs response validation so persisted unsafe snapshots are not
silently accepted.

The Server/API layer decides which completed responses are eligible for durable
replay. Core validates the snapshot shape but does not decide endpoint-specific
retention or HTTP caching policy.

## Operation log

Storage owns append-only persistence and must generate a globally monotonic,
non-negative `seq`. A durable row should preserve the Core operation identifier,
originating adapter, operation kind, normalized path, optional revision/tombstone/
conflict identifiers, and append timestamp.

`OperationKind` strings are exact V1 snake_case values. Unknown or differently
cased values are rejected. Sequence, changes-limit, tombstone-id, and changes-page
deserialization re-runs Core validation.

For a changes query:

- return rows strictly after `since`;
- order rows by ascending sequence;
- return no more than `limit` rows;
- fetch `limit + 1` internally when using the sentinel-row strategy;
- trim the sentinel before constructing `ChangesPage`;
- set `has_more` from the sentinel result;
- use the final returned row as `to_seq`, or `from_seq` for an empty page.

## Adapter cursor

Storage should keep one cursor per adapter. The adapter may advance its Core
cursor only after it has successfully applied every returned change through the
page `to_seq`.

Cursor persistence must:

- reject sequence regression;
- treat the same sequence as idempotent/unchanged;
- advance only to a higher successfully processed sequence;
- update `last_core_seq`, secret-free external cursor metadata, and
  `last_success_at` atomically;
- scope reads and writes to the owning adapter.

`external_cursor_json` is opaque to Core and therefore cannot be proven secret-free
by the classifier. Storage/adapter code must keep OAuth tokens, refresh tokens,
credentials, and raw provider payloads out of it.
