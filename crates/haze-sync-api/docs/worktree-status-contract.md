# Passive Worktree Status Contract

Phase: `API-P8-WORKTREE-STATUS-CONTRACT`

Accepted Server source: `1d1fc8ca62c97db041cca09dd8316370285dfba1`

## Ownership

`haze-sync-api` owns only the public JSON vocabulary, pure admin-role validation, secret-safe error mapping, deterministic fixtures, and passive response builders for future Server-owned surfaces:

```text
GET  /v1/admin/worktree/status
POST /v1/admin/worktree/sync-once
```

The routes are not registered in API. Server remains responsible for HTTP wiring, runtime status collection, manual-cycle submission, lifecycle policy, mode gating, and authentication lookup. Worktree remains responsible for filesystem and cycle execution. API does not access Server, Worktree, Storage, Core, providers, or the filesystem.

## Status JSON

The status response exposes exactly:

```text
configured_mode
host_lifecycle
readiness
readiness_reason
cycles_completed
cycles_failed
cycle_in_progress
pending_watcher_hints
manual_availability
```

Closed values:

```text
configured_mode:
  disabled | read_only | import_only | export_only | bidirectional | dry_run

host_lifecycle:
  disabled | starting | running | cancelling | shutdown | failed

readiness:
  ready | not_ready

readiness_reason:
  disabled_inert | running | starting | cancelling | shutdown | failed

manual_availability:
  available | busy | not_started | cancelling | shutdown | unavailable | failed
```

API accepts readiness, reason, and manual availability as already-sanitized Server facts. It does not recompute lifecycle or mode policy.

Accepted semantics:

- disabled is `ready` with `disabled_inert` and manual `unavailable`;
- running remains `ready` when manual availability is `busy`;
- starting, cancelling, shutdown, and failed are `not_ready`;
- counters and watcher hints are informational and do not determine readiness.

All public counts serialize as JSON-compatible `u64`. A checked helper converts a future platform-width watcher count without including the rejected value in an error.

## Sync-once submission

The request is the deterministic empty object `{}`. Unknown fields are rejected, so clients cannot supply paths, action budgets, force flags, mode overrides, request identifiers, runtime tickets, generations, or internal cycle requests.

Only an already verified `AdapterPrincipal` with `AdapterRole::Admin` is accepted by the pure authorization helper. Missing and non-admin principals map to the existing `missing_token` and `forbidden_role` public errors without identity or token data.

Closed submission outcomes:

```text
accepted | busy | not_started | cancelling | shutdown | unavailable | failed
```

`accepted` means Server accepted or queued the submission. It does not mean the cycle started or completed successfully. No completion, polling, wait, retry, ticket, or generation contract is present.

## Safety boundary

The DTOs cannot represent raw errors, paths, root fingerprints, database URLs, provider or request payloads, cursors, tokens, token hashes, idempotency values, runtime tickets, generation identifiers, file bytes, or backend details.

## Compatibility evidence

The deterministic fixture is:

```text
crates/haze-sync-api/fixtures/worktree-contract-v1.json
```

Its strict verifier is:

```text
crates/haze-sync-api/tests/worktree_compatibility_fixture.rs
```

The fixture covers the representative running/busy status response, the bodyless request, every closed vocabulary value, every submission outcome, safe admin authorization, semantic examples, and forbidden-field absence.
