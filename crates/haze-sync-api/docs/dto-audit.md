# Public DTO Audit Notes

Status: API-P2 implementation note

## Scope

This note records the public DTO safety audit for `haze-sync-api`.

Audited surfaces:

- server-info DTOs;
- changes feed DTOs;
- file metadata, upload outcome, download metadata, and delete outcome DTOs;
- conflict list/detail/resolve DTOs;
- safe public error DTOs;
- passive admin/status summary DTOs.

## Stable JSON vocabulary

Public enum and tagged-response vocabularies are intentionally `snake_case`.

Covered vocabularies include:

- operation kinds: `upsert_file`, `delete_file`, `restore_file`, `conflict_created`, `conflict_resolved`, `backup_created`;
- file PUT statuses: `accepted`, `conflict_saved`, `ignored`, `rejected`;
- file DELETE statuses: `tombstoned`, `not_found`, `rejected`;
- conflict actions: `accept_current`, `accept_conflict`, `keep_both`, `mark_resolved`;
- public error codes such as `invalid_request`, `invalid_path`, `validation_error`, `unauthorized`, `forbidden_role`, `not_found`, `idempotency_conflict`, and `unsafe_delete`.

## Safe output boundaries

JSON DTOs intentionally carry metadata only.

The public DTO contract does not represent:

- raw file bytes;
- raw request bodies;
- bearer tokens;
- token hashes;
- idempotency key values;
- database URLs;
- local filesystem roots;
- provider payloads;
- stack traces or backtraces;
- raw external cursors.

Content hashes remain public file-integrity metadata. They are distinct from token hashes and may appear as `content_sha256` values.

## Intentionally partial or placeholder DTOs

The admin/status DTOs remain summary-only and passive.

Current placeholder/partial areas:

- `StatusSummaryResponse::placeholder()` represents dependency-free status before server handlers wire real dependency checks.
- `PauseStatusSummary::unsupported()` represents that pause/resume support is not schema/runtime-backed yet.
- `AdapterCursorSummary` exposes cursor presence through `has_external_cursor`, but never the raw external cursor value.

The conflict detail DTO is also summary-only. It exposes identifiers, paths, policy/status vocabulary, and timestamps, but no current or incoming file bytes.

## Ownership boundary

DTOs do not decide Core policy, create revisions, create tombstones, resolve conflicts, update cursors, query storage, register routes, call providers, or spawn runtime work.

Server and fan-in phases remain responsible for mapping already-sanitized runtime/Core results into these public shapes.
