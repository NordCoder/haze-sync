# Implementation Plan: gdrive-adapter

## Current state

`haze-gdrive-adapter` is currently a placeholder binary.

Implemented current-state surface:

- binary package named `haze-gdrive-adapter`;
- `src/main.rs` entrypoint;
- process prints `haze-gdrive-adapter skeleton` and exits;
- no network services;
- no Google API calls;
- no credential reads;
- no sync behavior.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for the GDrive adapter is a safe Google Drive replica adapter that synchronizes a configured Drive folder subtree with Haze Sync Server/Core state.

The component is V1-ready when:

- OAuth/token loading is explicit, secret-safe, and outside the repo;
- adapter config defines Drive root, server URL, adapter identity, auth, mode, polling/full-scan cadence, and safety limits;
- Drive file metadata/content/change feed are normalized into Core/API facts;
- full scans are implemented as correctness mechanism;
- change feed/webhook support, if present, is only latency optimization;
- mapping/cursor/echo state is persisted through an accepted boundary;
- import/export/delete behavior respects adapter modes;
- Drive disappearances become guarded delete candidates before Core tombstones;
- mass delete conditions stop or block safely;
- provider errors are sanitized and retried/backed off predictably;
- no raw tokens/provider payloads/secrets appear in logs/status/errors.

## Implementation phases

### GDA-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold GDrive adapter docs with a real contract, dependency map,
implementation plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial component decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no Google API dependencies;
- no OAuth implementation;
- no sync behavior;
- no CI/workflow changes.

Acceptance:

- docs define GDrive as external replica adapter;
- docs preserve Core/API/Server/Storage/GDrive boundaries;
- future GDrive phases are implementable without local sync policy ownership.

### GDA-P2 — Config, secret loading, and runtime skeleton

Goal:

```text
Implement explicit adapter configuration, secret-path handling, safe redaction,
and process lifecycle foundation before provider calls are added.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- define config for server URL, adapter token, Drive root folder id, OAuth token path, adapter mode, dry-run, polling/full-scan intervals, and delete safety thresholds;
- load config from explicit environment/files without committing secrets;
- represent secret values with redacted debug/display behavior;
- add skeleton lifecycle with startup validation and graceful shutdown hooks;
- add safe status/error classification for config failures.

Non-goals:

- no Google API calls;
- no Core API calls;
- no mapping persistence;
- no sync loop;
- no token refresh implementation beyond safe loading if scoped.

Contract-change triggers:

- committing tokens or sample real credentials;
- logging secret paths/values unsafely;
- changing deployment secret layout without deployment docs;
- adding hidden direct DB dependency.

Acceptance:

- adapter starts/stops with explicit config;
- secret values are redacted;
- invalid config fails safely.

### GDA-P3 — Google client abstraction and provider-safe DTO normalization

Goal:

```text
Introduce a provider abstraction that can be tested without real Google access and
normalize Drive metadata into safe internal facts.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- define trait/interface for Drive list/get/download/upload/update/delete/trash operations;
- implement fake provider for tests;
- map Drive file metadata to provider-neutral adapter facts;
- identify supported file types for V1;
- explicitly skip unsupported Docs/Sheets/Slides/shortcuts/shared-drive cases unless contract changes;
- sanitize provider errors and raw payloads;
- add tests for metadata normalization and unsupported entry classification.

Non-goals:

- no real Google SDK wiring unless separately scoped in the same phase;
- no Core API writes;
- no mapping DB persistence;
- no provider delete actions.

Contract-change triggers:

- adding Docs/Sheets/Slides conversion;
- supporting shared drives/shortcuts;
- exposing raw provider payloads;
- relying on provider metadata as Core policy decision.

Acceptance:

- provider behavior is testable through fakes;
- normalized facts are safe and provider-neutral;
- unsupported files are skipped explicitly.

### GDA-P4 — Mapping, cursor, and echo-state boundary

Goal:

```text
Define and implement the adapter-side state needed to avoid duplicate imports,
track Drive/Core correspondence, and prevent echo loops.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
crates/haze-sync-storage/** only if explicit cross-component fan-in scopes it
crates/haze-sync-server/** only if explicit cross-component fan-in scopes it
```

Likely work:

- define adapter-local mapping model aligned with `gdrive_mapping` storage shape;
- track path, drive file id, parent id, name, checksum, Drive version/modified time, Core revision, Core seq, last imported/exported timestamps, last seen timestamp, delete candidate timestamp;
- define cursor progression model for Drive change feed and Core changes feed;
- implement echo guard for adapter-created Drive writes;
- choose persistence boundary: Server/API-mediated or Storage repository access only after architecture decision.

Non-goals:

- no direct DB access unless explicitly accepted;
- no provider sync loop;
- no Core policy;
- no hard delete.

Contract-change triggers:

- direct database ownership by adapter;
- changing `gdrive_mapping` schema;
- exposing raw external cursor JSON publicly;
- storing OAuth tokens in mapping/cursor state.

Acceptance:

- mapping/cursor/echo state ownership is explicit;
- echo loops have a testable prevention model;
- persistence boundary is documented.

### GDA-P5 — Full scan and import planner

Goal:

```text
Implement full Drive scan reconciliation and convert provider changes into safe
Core/API import requests.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- list configured Drive subtree;
- normalize folder/file paths to vault paths;
- detect new/modified/missing/unsupported entries;
- download supported file bytes when needed;
- compute content hash;
- prepare Core/API upload requests with base revision or explicit null base;
- detect Drive disappearance as delete candidate, not immediate delete;
- apply adapter mode rules before import;
- test with fake provider trees.

Non-goals:

- no Drive export;
- no real provider calls if fake-first phase;
- no hard delete;
- no Core policy decisions;
- no Docs conversion.

Contract-change triggers:

- treating Drive state as authoritative over Core;
- import without base/null-base semantics;
- tombstoning on first disappearance;
- path normalization divergence from Common/API.

Acceptance:

- full scan can produce safe import plans;
- unsupported/provider-specific entries are skipped safely;
- delete candidates are conservative.

### GDA-P6 — Change feed polling and reconciliation loop

Goal:

```text
Use Drive change feed as a latency layer while keeping full scans as correctness
backstop.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- poll Drive changes using cursor state;
- classify change feed entries into scan/import/export work;
- handle cursor invalidation by falling back to full scan;
- debounce/coalesce changes safely;
- persist cursor only after successful processing;
- integrate retry/backoff for provider rate limits/errors;
- test cursor invalidation and duplicate/reordered change entries.

Non-goals:

- no webhook/public callback infrastructure unless scoped;
- no change-feed-only correctness;
- no provider deletion side effects;
- no direct DB writes without accepted boundary.

Contract-change triggers:

- claiming change feed alone is sufficient for correctness;
- advancing cursor before successful processing;
- losing changes on cursor invalidation;
- exposing raw cursor/provider payloads publicly.

Acceptance:

- change feed improves latency but full scan remains authoritative;
- cursor behavior is safe and testable;
- provider errors are backoff-classified.

### GDA-P7 — Core changes export planner and Drive apply runner

Goal:

```text
Export accepted Core/API changes to Google Drive while preserving echo guard,
mode rules, and provider-safe error handling.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- read Core/API changes since last Core seq;
- plan Drive creates/updates/trashes from accepted Core revisions/tombstones;
- upload bytes to Drive only after content hash/source verification;
- update mapping and echo state after provider confirmation;
- respect export_only/bidirectional/dry_run modes;
- handle provider conflicts/rate limits/retries safely;
- test with fake provider and fake Core client.

Non-goals:

- no Core conflict policy;
- no Drive hard delete by default;
- no Docs conversion;
- no plugin/worktree behavior.

Contract-change triggers:

- provider overwrite without Core/API confirmed change;
- hard delete instead of trash/retention behavior;
- exposing provider payloads;
- updating mapping before provider confirmation.

Acceptance:

- Drive export is provider-safe and echo-guarded;
- mode enforcement prevents unintended mutations;
- mapping state follows confirmed provider outcomes.

### GDA-P8 — Delete candidate guardrails and mass-delete safety

Goal:

```text
Prevent Drive-side disappearance or provider anomalies from causing unsafe Core or
Drive mass deletes.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
```

Likely work:

- classify missing Drive files as delete candidates with timestamp/state;
- require confirmation across scan/change cycles before submitting delete;
- integrate Core/API delete guard semantics and adapter-specific thresholds;
- stop/alert on many deletions or high ratio;
- support manual unlock/override only if accepted by Core/API/Server contracts;
- test provider folder disappearance, auth scoping loss, and mass-delete scenarios.

Non-goals:

- no immediate tombstone on disappearance;
- no Drive hard delete;
- no admin mutation route unless scoped elsewhere;
- no bypass of Core delete policy.

Contract-change triggers:

- mass delete propagation without guard;
- adapter-local delete policy divergent from Core;
- manual override without audit/contract;
- hard delete provider calls.

Acceptance:

- delete candidates are conservative and observable;
- unsafe mass-delete conditions block sync;
- Core remains delete arbiter.

### GDA-P9 — Status, doctor, observability, and E2E readiness

Goal:

```text
Make adapter behavior diagnosable and ready for local integration testing without
leaking secrets or provider payloads.
```

Allowed scope:

```text
crates/haze-gdrive-adapter/**
```

Likely work:

- expose safe status summary: mode, last scan, last change-feed poll, last import/export, cursor presence, pending delete candidates, last safe error category;
- provide doctor checks for auth configured, Drive root reachable, mapping consistency, cursor health, delete guard state, and server connectivity;
- add structured logs/metrics with redaction if scoped;
- add fake-provider E2E tests for import/export/delete-candidate loops;
- document local runbook and secret file expectations.

Non-goals:

- no real production Drive credentials in tests;
- no provider payload snapshots;
- no deployment service files unless deployment component scopes them;
- no hard delete.

Contract-change triggers:

- status exposing tokens/raw cursors/provider payloads;
- tests requiring real credentials;
- deployment changes outside component scope;
- doctor repair actions that mutate provider/Core state without explicit contract.

Acceptance:

- adapter can be diagnosed safely;
- fake-provider tests cover critical flows;
- production rollout blockers are explicit.

## Dependency gates

GDrive adapter implementation depends on stable Common/API/Core/Server/Storage contracts:

- Common owns path/hash/adapter primitive semantics.
- API owns HTTP DTO/header/error vocabulary.
- Core owns conflict/delete/revision/idempotency policy.
- Server exposes runtime API and safe status/errors.
- Storage owns `gdrive_mapping`/cursor persistence if selected.

Mapping/cursor persistence and direct DB access must be decided before implementation assumes Storage ownership.

## Known risks

- OAuth tokens and provider payloads are high-risk leak sources.
- Drive change feed can be incomplete/invalidated; full scan is required for correctness.
- Drive disappearances can represent permission loss, folder move, provider glitch, or delete; tombstoning immediately is unsafe.
- Echo guard must distinguish adapter-created writes from real user edits.
- Google Docs/Sheets/Slides/shortcuts/shared drives can blur V1 scope.
- Direct DB access from adapter can bypass Server/Core/API boundaries.
- Provider rate limits and retries can create duplicate mutations without idempotency discipline.
- Mapping drift can cause wrong-file updates if not detected.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI;
- execute GDA-P2 through GDA-P9 implementation/clean-code/CI phases;
- add Google API/OAuth dependencies;
- implement provider client;
- implement Core API client;
- decide mapping/cursor persistence boundary;
- add fake-provider tests;
- run against real Drive credentials;
- deployment/service setup;
- provider webhook support, if ever scoped.

## Completion criteria for the component

`haze-gdrive-adapter` is V1-ready when:

- config/secrets/lifecycle are explicit and redacted;
- provider metadata is normalized safely;
- full scan and change-feed loops are correct and resilient;
- import/export flows preserve Core/API base/idempotency/conflict/delete semantics;
- mapping/cursor/echo state is persisted through an accepted boundary;
- delete candidates and mass-delete guardrails prevent provider accidents from causing data loss;
- status/doctor/observability are safe and useful;
- tests cover fake-provider import/export/delete scenarios without real secrets;
- no plugin/worktree/Core policy/Storage schema ownership leaks into the adapter.
