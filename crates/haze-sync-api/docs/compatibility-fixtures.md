# API V1 Compatibility Fixtures

## Purpose

`fixtures/api-contract-v1.json` is the language-neutral compatibility reference for the established public JSON shapes owned by `haze-sync-api`.

`fixtures/worktree-contract-v1.json` is the isolated compatibility reference for the API-P8 hosted Worktree status and bodyless sync-once submission contracts. Keeping it separate makes the new Server-pinned surface reviewable without redefining the accepted API-P7 fixture groups.

The fixtures exist so Rust Server/CLI code and TypeScript clients can detect drift in:

- server-info metadata and capability names;
- file metadata;
- all tagged PUT and DELETE outcome shapes;
- changes-page fields and operation-kind vocabulary;
- the Server-exposed conflict list shape and conflict resolution body/response;
- sanitized public error envelopes;
- admin status, adapter, doctor, and adapter-runtime summaries;
- Worktree mode, lifecycle, readiness, readiness-reason, manual-availability, and submission vocabularies;
- the secret-safe Worktree status snapshot and bodyless sync-once request/response shapes;
- closed API-owned string vocabularies.

The general fixture is verified by `tests/compatibility_fixtures.rs`. The Worktree fixture is verified by `tests/worktree_compatibility_fixture.rs`. Both verifiers use current Rust DTOs and passive route helpers only.

## Scope

The fixtures contain synthetic public contract data only. They do not define or execute:

- HTTP routing, authentication lookup, persistence, Core policy, or provider behavior;
- request headers, bearer tokens, idempotency-key values, or file bytes;
- database/object-store configuration or local filesystem paths;
- Worktree scanning, writing, runtime cycles, task submission, polling, retries, waits, tickets, or completion;
- generated clients or TypeScript runtime behavior.

Common-owned primitive normalization and validation remain covered by `crates/haze-sync-common/fixtures/common-primitives-v1.json`. API fixtures embed already canonical primitive strings inside API-owned envelopes.

## General fixture structure

`schema_version` identifies the fixture document schema. Version `1` of `api-contract-v1.json` contains:

- `server_info`;
- `file_metadata`;
- `put_file_outcomes`;
- `changes_page`;
- `conflict_list_query` and `conflict_list`;
- `conflict_resolutions`, including the path conflict id plus passive API request/response JSON;
- `delete_file_outcomes`;
- `public_errors`;
- `admin` summaries;
- `vocabulary`, containing complete closed wire-value sets represented through API-P7.

The conflict-list fixture follows the route contract currently consumed by Server: `ConflictListRouteResponse` with `conflict_id`, `original_path`, `conflict_path`, revision fields, `source_adapter_id`, `policy_applied`, status, and safe timestamps.

The four `conflict_resolutions` entries verify API-owned request vocabulary and passive response-builder JSON only; they are not a Server support matrix. In particular, `accept_conflict` is accepted by the public API vocabulary but remains reserved for a future promotion flow, and the current Server returns a sanitized not-implemented response instead of executing it. Clients must handle runtime support independently from DTO compatibility.

The `admin` group is one coherent synthetic snapshot: `status_summary.adapter_count`, `adapter_list.total_count`, adapter identities, and `adapter_operational_summaries` refer to the same two adapters.

## Worktree fixture structure

Version `1` of `worktree-contract-v1.json` contains:

- `status`, a complete representative running/ready/busy snapshot;
- `sync_once_request`, the deterministic empty request object `{}`;
- `sync_once_outcomes`, containing each submission-only outcome exactly once;
- `vocabulary`, containing complete mode, lifecycle, readiness, reason, manual-availability, and submission-status sets.

The fixture is pinned conceptually to the accepted Server status vocabulary at code-bearing SHA `1d1fc8ca62c97db041cca09dd8316370285dfba1`, but it does not import Server types or claim Server route wiring exists.

`accepted` means the future Server handler accepted or queued a manual submission. It is not a completion result. The fixture intentionally has no path, force flag, mode override, budget, request id, ticket, generation, error detail, or backend payload.

## Verification rules

The Rust verifiers require:

- recognized `schema_version`;
- no unknown root or fixture-group fields;
- exact deserialize/serialize equality for every example;
- complete and unique closed vocabularies compared as unordered sets;
- valid server-info metadata;
- valid changes pagination metadata;
- conflict query/resolution examples accepted by passive route helpers;
- every tagged PUT/DELETE status and every conflict-resolution action to appear exactly once in their representative fixture groups;
- status/readiness/timestamp combinations to be internally consistent;
- pause summaries and adapter runtime summaries to be internally consistent;
- admin adapter counts and adapter identities to agree across status, list, and operational summaries;
- disabled Worktree to remain ready/inert and manually unavailable;
- running/busy Worktree status to remain ready regardless of informational counters;
- failed Worktree status to remain not-ready;
- every sync-once submission status to appear exactly once;
- only a verified admin principal to satisfy the pure sync-once authorization helper;
- no environment-specific, provider-specific, secret-bearing, filesystem, ticket, generation, or raw runtime values.

Array order is for readability unless an endpoint explicitly defines response order. Vocabulary arrays should be treated as sets by downstream compatibility tests.

## TypeScript mirror guidance

The Obsidian plugin should use these fixtures in OBS-P9 tests to validate its manually maintained DTO mirror. This phase does not edit or generate TypeScript.

Recommended rules:

- Validate `schema_version` before consuming either fixture.
- Use exact `snake_case` JSON field names. Do not rename wire fields in DTO interfaces.
- Model PUT and DELETE responses as discriminated unions on `status`; variant-only fields must not be treated as universally present.
- Model changes with `from_seq`, `to_seq`, `has_more`, and entries using `seq`, `kind`, `content_sha256`, `updated_by`, and `updated_at`.
- Treat absent optional fields differently from explicit `null`. For example, optional change metadata is omitted, while fields whose contract explicitly represents an unknown value may be `null`.
- Model the conflict list using the fixture’s route fields. The resolve request body is `{ "resolution": <action> }`; the conflict id belongs to the route path, not the JSON body.
- Treat conflict-resolution vocabulary as DTO compatibility, not proof that every action is executable by the current Server. `accept_conflict` is reserved and currently returns a sanitized not-implemented response; clients must not enable it solely because it appears in the fixture vocabulary.
- Treat `ErrorResponse.error.code` as a closed public code vocabulary in compatibility tests. `request_id` and `details` are optional, and `details` may be a string-list map or a flat string list.
- Treat server capability, operation kind, conflict action/status, delete reason, readiness, doctor, cursor-presence, adapter-runtime, and Worktree values as exact literal unions. Avoid `| string` fallbacks in compatibility tests because they hide drift.
- Treat the Worktree `readiness`, `readiness_reason`, and `manual_availability` fields as Server-supplied facts. Clients must not recompute lifecycle or mode policy from counts.
- Treat `sync_once_request` as exactly `{}` and `accepted` as submission-only. Do not infer cycle completion, polling, ticket, retry, force, path, or mode-override behavior.
- Validate sequence and count values with `Number.isSafeInteger` before storing or comparing them in TypeScript. The fixtures use small deterministic integers and do not redefine the Rust integer contracts.
- Reuse the Common fixture for path/hash/identifier and adapter role/mode validation rather than inferring primitive rules from API examples.
- Do not infer client retry, overwrite, conflict-resolution, cursor advancement, local-file, or Worktree runtime behavior from these examples. Those behaviors remain owned by the client component and accepted Server/Core/Worktree contracts.

A TypeScript compatibility test should parse each fixture value through the plugin’s DTO validator and compare the re-serialized JSON shape to the fixture. Merely accepting the JSON without checking exact keys, discriminator values, and omitted/null behavior is not sufficient drift detection.

## Versioning

Breaking changes to V1 field names, discriminator values, or represented wire semantics require an explicit API contract change and a new versioned fixture rather than silently redefining an existing file.

A non-breaking additional example may update a V1 fixture only when its Rust compatibility test changes in the same code-bearing phase and existing represented shapes retain their meaning.

## Security

Compatibility fixtures must remain safe to publish, log, and copy into client tests. They must not contain:

- real credentials, bearer/OAuth material, token hashes, or idempotency-key values;
- database URLs, HTTP endpoints, local absolute paths, roots, or environment hostnames;
- raw errors, stack traces, provider payloads, external cursor values, request bodies, runtime tickets, or generation identifiers;
- real user vault content or production identifiers.
