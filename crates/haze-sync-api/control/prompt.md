# W1-API-P8-WORKTREE-STATUS-CONTRACT — Passive status and sync-once contract

Before starting, name this worker chat exactly:

`api — W1 API-P8 Worktree Status Contract`

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker
Phase: API-P8-WORKTREE-STATUS-CONTRACT

Do not merge, change draft state, rewrite history, modify sibling branches, or implement Axum/runtime behavior.

## Accepted baselines

API branch synchronization:

- synchronized code-bearing merge SHA: `d0e8ef0705b7c0456f2cb1359428ff30b90961b4`;
- exact main merged: `c1e69a664388b0cba028170e8398b9088218957d`;
- Component CI run `29312597987`, number `1913`, success;
- API-P7C accepted baseline before sync: `3109c0fd9b456ca5fd8db099cd83843dae44cef9`.

Accepted Server contract:

- SRV-P7B5 code-bearing SHA: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- Server clean report commit: `23b49dff38c8b6997193b6224681feada3e09c1e`;
- Server clean report blob: `2416d7280883761bf90117a7e3dbfc41147756b9`;
- status: `CLEAN_ACCEPT`.

## Goal

Define the stable, JSON-serializable, secret-safe public API contract that Server can later wire to its accepted internal Worktree status snapshot and bounded manual submission boundary.

API remains passive. It owns DTOs, pure validation/auth helpers, safe error/response vocabulary, serialization tests and compatibility fixtures only.

No Server, Worktree, Storage, Core, CLI or Deployment product behavior may be implemented here.

## Status contract

Add an isolated Worktree status DTO/helper surface, preferably under dedicated `dto/worktree.rs` and `routes/worktree.rs` modules unless the existing API structure clearly supports a smaller equivalent.

The public response must represent exactly these accepted safe fields:

- configured mode;
- host lifecycle;
- readiness category;
- readiness reason;
- cycles completed;
- cycles failed;
- cycle-in-progress boolean;
- pending watcher hint count;
- manual availability.

Required stable `snake_case` vocabulary:

- mode: `disabled`, `read_only`, `import_only`, `export_only`, `bidirectional`, `dry_run`;
- lifecycle: `disabled`, `starting`, `running`, `cancelling`, `shutdown`, `failed`;
- readiness: `ready`, `not_ready`;
- readiness reason: `disabled_inert`, `running`, `starting`, `cancelling`, `shutdown`, `failed`;
- manual availability: `available`, `busy`, `not_started`, `cancelling`, `shutdown`, `unavailable`, `failed`.

Semantics that DTO docs/tests must preserve:

- Disabled is Ready / DisabledInert and manually Unavailable.
- Starting, Cancelling, Shutdown and Failed are NotReady.
- Running is Ready even while manual availability is Busy.
- Counters and watcher-hint counts are informational only and never determine readiness.
- Manual availability is an already-sanitized Server result; API must not recalculate Worktree gate/lifecycle policy.
- No raw error, path, root fingerprint, database URL, provider payload, request payload, cursor, token, idempotency value or backend detail is representable.
- Public integer count types must have stable JSON behavior; checked conversion helpers may reject impossible platform-width conversions without exposing internals.

Provide constructors/builders from already-sanitized public parts only. Do not add a dependency on `haze-sync-server` or import private Server enums.

## Sync-once submission contract

Add a narrow administrative submission contract for a future Server-owned endpoint, with no runtime execution in API.

Recommended future HTTP surfaces for documentation/route-helper naming:

- `GET /v1/admin/worktree/status`;
- `POST /v1/admin/worktree/sync-once`.

Do not register these routes.

The sync-once contract must:

1. Require an already verified `AdapterPrincipal` with `AdapterRole::Admin` through a pure helper.
2. Expose no public action budgets, filesystem paths, mode override, force flag, request id, runtime ticket, generation or internal cycle request.
3. Model submission only, not completion. `accepted` means Server accepted/queued the request; it does not mean the cycle completed successfully.
4. Use stable submission outcomes:
   - `accepted`;
   - `busy`;
   - `not_started`;
   - `cancelling`;
   - `shutdown`;
   - `unavailable`;
   - `failed`.
5. Preserve the accepted Server rule that manual submission may be unavailable outside the configured mode; API must not infer or override that policy.
6. Map missing/unauthorized principals through existing safe auth/public-error vocabulary without leaking identity/token data.
7. Keep runtime rejection/outcome mapping pure and coarse. No raw Worktree failure codes or contract-violation internals should become public in this phase.

A bodyless request marker or deterministic empty request object is acceptable. Prefer the smallest contract that does not imply client-controlled runtime policy.

## Compatibility and tests

Add deterministic tests for:

- every status enum JSON value and roundtrip;
- a complete representative status response JSON fixture;
- Disabled ready/inert output;
- Running + Busy still ready;
- Failed not-ready output;
- counters/hints preserved but not used to derive readiness;
- all sync-once submission outcome JSON values;
- accepted means submitted, not completed;
- admin principal accepted;
- non-admin and missing principal rejected safely;
- absence of raw paths, root fingerprints, database URLs, raw errors, payloads, tokens, hashes, cursors, idempotency material and ticket/generation identifiers;
- passive behavior: no DB, filesystem, provider, runtime, task, polling, wait loop or route registration.

Update the API compatibility fixture and fixture tests when the existing fixture format can represent the new stable contract without editing downstream components.

Update relevant API docs to record API-P8 and the Server-owned runtime boundary.

## Allowed scope

- `crates/haze-sync-api/src/dto/**`;
- `crates/haze-sync-api/src/routes/**`;
- `crates/haze-sync-api/src/contracts/errors.rs` only if a genuinely necessary coarse public code cannot use the existing safe vocabulary;
- `crates/haze-sync-api/src/auth/**` only for a minimal pure admin-role helper if existing `can_admin()` is insufficient;
- `crates/haze-sync-api/fixtures/**`;
- `crates/haze-sync-api/tests/**`;
- `crates/haze-sync-api/docs/**`;
- API control report.

## Forbidden scope

- Axum handlers/router registration/middleware/listener code;
- Server app state or Server route changes;
- Worktree runtime, gate, watcher, scheduler or ticket changes;
- database/object-store/provider/filesystem calls;
- background tasks, polling, retries or waits;
- CLI or Deployment changes;
- migrations, workflows or dependency expansion unless compilation proves a minimal existing-crate dependency declaration is missing;
- public raw failure details or secret-bearing fields;
- unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip. Obtain authoritative Component CI on the exact final code-bearing SHA.

Write `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: API-P8-WORKTREE-STATUS-CONTRACT`;
- `chat_name: api — W1 API-P8 Worktree Status Contract`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record changed paths, exact public JSON vocabulary, auth/submission semantics, fixture/test coverage, final SHA and exact CI evidence.

Do not claim CLEAN_ACCEPT, do not wire Server, and do not begin CLI-P6A. A focused API-P8 functional review follows.
