# Component Contract: gdrive-adapter

## Responsibility

`haze-gdrive-adapter` owns the Google Drive replica adapter/runtime for Haze Sync.

The adapter synchronizes a configured Google Drive folder subtree with the authoritative Haze Sync Server/Core state through provider-neutral Core/API contracts.

The component is responsible for future behavior such as:

- Google OAuth/token loading from configured secret files;
- Google Drive API client construction;
- Drive folder/file discovery and mapping;
- Drive change feed polling and full-scan reconciliation;
- Drive import of new/modified/deleted Markdown/files into Haze Sync Core/API;
- Drive export of Core/API changes back to Google Drive;
- Drive echo guard and mapping updates;
- delete-candidate detection with conservative guardrails;
- dry-run/import-only/export-only/bidirectional mode behavior;
- provider-safe retries/backoff/status/doctor facts;
- safe logging and operator summaries.

The adapter submits provider facts to Core/API and obeys Core/API outcomes. It must not decide conflict/delete/revision policy locally.

Current code state is a placeholder binary that prints a skeleton message and performs no network, credential, provider, or sync behavior.

## Public interfaces

Current public/runtime interface:

```text
Binary: haze-gdrive-adapter
Entrypoint: src/main.rs
Current behavior: print skeleton message and exit
```

Target internal interfaces may include:

```text
GDriveAdapterConfig
GDriveAuthConfig
GDriveClient
DriveFileMapping
DriveChangeScanner
DriveFullScanner
DriveImportPlanner
DriveExportPlanner
DriveApplyRunner
DriveEchoGuard
DriveDeleteGuard
DriveCursorStore
GDriveStatusSummary
GDriveDoctorSummary
BackoffPolicy
```

Names are indicative. Implementation workers should choose final names consistent with Rust crate style and current upstream contracts.

## Input contracts

Adapter inputs come from:

- configured Google Drive folder root and OAuth credentials;
- Google Drive file metadata/content/change feed;
- Haze Sync Server/Core API changes and outcomes;
- persisted mapping/cursor state through accepted boundary;
- adapter mode/configuration;
- operator lifecycle commands or process signals.

Required input rules:

- OAuth refresh/access tokens are secrets and must never be logged, committed, or exposed in status output;
- Drive file IDs, parent IDs, versions, checksums, and modified times are provider facts, not sync policy decisions;
- Drive changes must be normalized into vault-relative paths and Core/API-compatible facts;
- write/delete submissions to Haze Sync must include content hash and base/null-base semantics where required;
- Drive disappearance/deletion must be treated as delete candidate first, not immediate tombstone propagation;
- full scans are required for correctness even if change feed/webhook support exists.

## Output contracts

Adapter outputs include:

- Haze Sync API write/delete requests;
- Google Drive create/update/delete/trash requests when Core/API export decisions require them;
- mapping/cursor updates through accepted persistence boundary;
- safe status/doctor summaries;
- safe logs/metrics if scoped.

Required output rules:

- public/operator output must not include OAuth tokens, bearer tokens, token hashes, idempotency keys, raw provider payloads, database URLs, local absolute paths, or raw response bodies;
- provider payloads must be summarized before status/log/report output;
- Drive exports must be echo-guarded so adapter-created provider updates are not re-imported as remote edits;
- delete propagation must be conservative and Core-confirmed;
- errors should be classified into safe categories such as auth, rate_limit, provider_unavailable, mapping_conflict, rejected_by_core, skipped_unsafe, and internal.

## Error contracts

GDrive adapter errors must be safe for logs, status surfaces, reports, and operator diagnostics.

Errors must not expose:

- OAuth refresh/access tokens;
- Haze Sync bearer tokens;
- token hashes;
- Idempotency-Key values;
- raw Google API request/response payloads containing sensitive data;
- raw file content;
- database URLs;
- local absolute secret paths;
- stack traces;
- raw provider error bodies without sanitization.

Provider request IDs and Drive file IDs may be operationally useful but must be reviewed before public exposure. Prefer safe summaries and correlation IDs.

## Persistence/runtime ownership

GDrive adapter owns provider runtime behavior.

Adapter may own:

- OAuth token file loading from a configured secret path;
- Google Drive client setup;
- provider polling/full-scan loop;
- provider retry/backoff;
- provider-to-Core normalization;
- Drive import/export planning;
- echo guard behavior;
- provider-safe status/doctor facts.

Adapter must not own:

- Core conflict/delete/revision policy;
- API DTO/header/public error vocabulary;
- Storage schema or direct database writes unless explicitly accepted by architecture;
- Server startup/lifecycle unless the adapter is intentionally hosted by Server in a future design;
- Obsidian plugin behavior;
- Worktree scanner/materializer behavior;
- hard delete of Drive files without explicit Core/API/operational contract.

Mapping/cursor persistence boundary is unresolved by default: the adapter may use Server/API-mediated persistence or Storage repository helpers only after an explicit integration decision.

## Security and secrecy rules

- Do not commit OAuth tokens, bearer tokens, client secrets, downloaded provider payloads, logs, dumps, or local secret paths.
- OAuth refresh token must live only in configured secret storage outside the repo.
- Do not log token values or raw Authorization headers.
- Do not expose raw Google API payloads in public status or reports.
- Do not directly touch Obsidian vault internals or Worktree files.
- Do not silently overwrite Core state or Drive files without Core/API-confirmed semantics.
- Do not propagate mass Drive deletions without delete-candidate guardrails and Core/API delete policy.
- Dry-run/import-only/export-only/bidirectional modes must be enforced before provider/Core mutations.

## Non-goals

GDrive adapter must not implement:

- Core sync policy decisions;
- Haze Sync HTTP server routes;
- API DTO ownership;
- Storage schema/migration ownership;
- direct database access by default;
- Obsidian plugin UI/local vault behavior;
- Worktree filesystem adapter behavior;
- Google Docs/Sheets/Slides document conversion in V1 unless a future contract changes non-goals;
- shared drives/shortcuts unless a future contract accepts them;
- semantic markdown merge;
- hard delete cleanup without explicit retention/delete contract.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- Core is authoritative; GDrive is an external replica.
- Adapter submits provider facts; Core/API decides conflict/delete/revision outcomes.
- Full scan is required for correctness; Drive change feed/webhooks are latency optimizations.
- Drive disappearance is a delete candidate before Core tombstone propagation.
- Many deletes must trigger guard/stop behavior rather than mass tombstone propagation.
- Echo guard must prevent adapter-created Drive writes from being re-imported as remote changes.
- Adapter mode must be respected before import/export/delete actions.
- Tokens and raw provider payloads must never appear in public outputs.

## Test obligations

GDrive adapter tests should eventually cover:

- config parsing and token-path redaction;
- OAuth/provider error sanitization using fake payloads;
- Drive metadata normalization into vault paths and mapping facts;
- unsafe file/folder/shortcut/shared-drive exclusions where scoped;
- import planning with base/null-base semantics;
- export planning with echo guard;
- delete candidate detection and mass-delete guard behavior;
- cursor/change-feed handling and full-scan reconciliation;
- mode behavior: disabled, dry_run, import_only, export_only, bidirectional;
- retry/backoff classification;
- absence of tokens, provider payloads, idempotency keys, and raw file content in logs/status/errors.

Expected checks when shell or CI is available:

```bash
cargo fmt --check
cargo check -p haze-gdrive-adapter
cargo test -p haze-gdrive-adapter
```

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- direct database writes from the adapter;
- provider policy decisions that bypass Core/API;
- plugin/worktree behavior inside the adapter;
- Google Docs/Sheets/Slides conversion;
- shared drive or shortcut support;
- hard delete of Drive files;
- exposing OAuth tokens/raw provider payloads;
- changing API/Core base revision, conflict, tombstone, or idempotency semantics;
- adding Server-hosted lifecycle instead of standalone adapter binary.
