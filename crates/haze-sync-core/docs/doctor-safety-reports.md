# Passive Doctor and Safety-Report Contract

## Ownership

`haze-sync-core::doctor` owns deterministic classification and aggregation over
already-redacted diagnostic facts. It does not open a database connection, probe
an object store or worktree, call a provider, inspect OAuth credentials, advance a
cursor, repair mappings, or mutate state.

Server, CLI, Storage, and adapter components own live checks. They must pass only
safe booleans, counts, content hashes, and fixed reason codes into Core.

## Check identifiers

The V1 passive model covers:

- `db_connectivity`;
- `object_store_exists_writable`;
- `missing_blobs`;
- `adapter_cursors`;
- `gdrive_mapping`;
- `adapter_token_sanity`;
- `worktree_drift`.

Cursor, mapping, token, and worktree summaries contain aggregate counts only. They
must not include adapter tokens, token hashes, provider file IDs, provider
payloads, local paths, database URLs, or raw errors.

## Status semantics

- `ok` — the check completed successfully;
- `warning` — the check completed and found a recoverable concern;
- `failed` — the check completed and found a blocking or integrity problem;
- `skipped` — the check was intentionally omitted because it was disabled or not
  applicable;
- `not_run` — the check is supported but was not attempted or did not provide a
  complete result;
- `placeholder` — the check is accepted by contract but downstream integration is
  still pending.

`skipped`, `not_run`, and `placeholder` are distinct. A tooling-limited caller must
not report `ok` or `skipped` when a supported check was simply not executed.

Aggregate precedence is:

```text
failed > warning > placeholder > not_run > skipped > ok
```

An empty report has `not_run` status because zero checks are not evidence of a
healthy system. Summary counts preserve every explicit status category.

## Classification rules

- DB metadata missing is a warning. Offline mode is skipped. A requested live
  check with no result is not-run. A negative live result is failed.
- Object-store configuration missing is a warning. No filesystem facts is the
  offline/skipped shape. Partial facts are not-run. Missing or non-writable storage
  is failed.
- Any missing required blob is failed. Only sorted, deduplicated content-hash
  samples may be exposed.
- Invalid adapter cursors are failed. Missing, orphaned, or stale cursors are
  warnings. No expected adapters and no cursors is skipped only when no invalid or
  stale cursor facts were supplied.
- GDrive mapping references to unknown Core revisions or duplicate provider file
  identifiers are failed. Unmapped-item drift is warning. An unconfigured adapter
  is skipped.
- Invalid enabled-adapter roles are failed. Missing token-hash metadata is warning.
  No enabled adapters is skipped. Raw token material is never accepted.
- Worktree references to unknown Core revisions are failed. Missing, mismatched,
  or unexpected materialized paths are warning. An unconfigured worktree is
  skipped.

Facts that could only come from an unperformed check are normalized away. In
particular, DB connectivity is omitted when metadata is not configured, object
store facts are omitted when the store is not configured, and mapping/worktree
counts are zeroed when their integration is disabled. This prevents a skipped or
configuration-warning result from carrying contradictory live-check evidence.

## Safe construction and serialization

Doctor result messages come from a fixed redacted vocabulary. Public callers
cannot inject arbitrary message text into a result. Deserialization rejects:

- message/status/check mismatches;
- details tagged for another check;
- detail counts or booleans that contradict the declared status and message;
- non-deterministic missing-blob samples;
- derived cursor counts that do not match expected and stored cursor totals.

Generic `not_run` and `placeholder` results expose only enum reason codes. A
serialized report is revalidated on deserialization: its summary must exactly
match the contained checks, and checks are deterministically ordered by stable
wire identifier.

Some report/result fields remain public for accepted CLI source compatibility.
They are compatibility views, not an escape hatch from invariants. Serialization
revalidates each result, deterministic ordering, and the aggregate summary; a
caller mutation that creates an inconsistent value returns a serialization error
rather than emitting misleading JSON.

## Downstream obligations

- CLI and Server may render Core results but own output formatting and exit/HTTP
  policy.
- Storage and adapters perform the actual checks and keep raw diagnostics private.
- A caller must not claim a live check was performed unless it actually obtained
  the corresponding facts.
- Public compatibility fields should be treated as read-only; construct reports
  through `DoctorReport::from_results` and use the provided classifiers.
- Repair, cursor advancement, mapping changes, cleanup, and provider actions remain
  separate explicit operations outside Core.
