# W1-SRV-API-P8-WORKTREE-HTTP-FAN-IN — Wire accepted Worktree admin contract

Before starting, name this worker chat exactly:

`server — W1 API-P8 Worktree HTTP Fan-In`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-API-P8-WORKTREE-HTTP-FAN-IN

Do not merge, change draft state, rewrite history, modify sibling branches, begin CLI-P6A, or perform unrelated cleanup.

## Accepted inputs

Server baseline:

- accepted SRV-P7B5 code-bearing SHA: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- Server clean report commit: `23b49dff38c8b6997193b6224681feada3e09c1e`;
- Server clean report blob: `2416d7280883761bf90117a7e3dbfc41147756b9`;
- DB-capable Component CI run `29289080020`, number `1912`, success.

Accepted API-P8 source:

- code-bearing SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- API clean report commit: `abbfb0c09f62d9f778a90207609318bda85de9ba`;
- API clean report blob: `401dc1c2fe9d5ce91a1aa248be8ed7d27a8275d7`;
- Component CI run `29315949762`, number `1925`, success.

Accepted API-P8 product blobs:

- `crates/haze-sync-api/src/dto/worktree.rs` → `7c592184d58c1cda98314fd0e2eae8baed1de1e8`;
- `crates/haze-sync-api/src/dto/mod.rs` → `5534f79606639fb13857de0729793d9083523e03`;
- `crates/haze-sync-api/src/routes/worktree.rs` → `032f43a1a513e7fb25d6103de281f7e5d8e08930`;
- `crates/haze-sync-api/src/routes/mod.rs` → `439bebd09d3aa9252188d2d3eb106e65943c5846`;
- `crates/haze-sync-api/fixtures/worktree-contract-v1.json` → `da0a197b1e6d5425f05cfd6fe772a4c96f6a830c`;
- `crates/haze-sync-api/tests/worktree_compatibility_fixture.rs` → `968f1e9825a2c54385615bf75db54a975218fd20`;
- `crates/haze-sync-api/docs/worktree-status-contract.md` → `f2c8d2d53c09a0e1ac0caf3ac8c6ee8d15754009`.

## Goal

Perform exact API-P8 product fan-in and wire these Server-owned HTTP surfaces:

- `GET /v1/admin/worktree/status`;
- `POST /v1/admin/worktree/sync-once`.

Both routes are administrative and require an already verified Admin principal.

API owns public DTO/error/authorization vocabulary. Server owns runtime state, HTTP status mapping, Worktree submission and lifecycle policy.

## Exact fan-in

Copy the accepted API-P8 product files needed by the workspace exactly from SHA `56ae945...`.

Requirements:

1. Copied API source/fixture/test/doc blobs must match the accepted blob SHAs exactly.
2. Do not copy API control files.
3. Do not semantically edit accepted API-P8 code on the Server branch.
4. If a listed non-code doc is unnecessary for compilation, copying it exactly is still allowed for contract evidence; do not rewrite it.

## Runtime control boundary

The current `ServerWorktreeRuntimeHost` owns shutdown and join lifecycle and must remain uniquely owned by startup.

Add the smallest cloneable Server-owned HTTP control handle, or equivalent, with these properties:

- created from the live host after startup;
- can obtain the existing passive `ServerWorktreeStatusSnapshot`;
- can make one bounded manual DryRun submission through the existing Worktree manual handle;
- carries only the Server-owned default manual budget/config needed to construct `WorktreeRuntimeManualRequest::dry_run(...)`;
- owns no task, join handle, shutdown sender, watcher, executor, DB pool, object store or filesystem root;
- does not use a mutex around the long-running host;
- does not duplicate Worktree gate/lifecycle accounting;
- does not poll or wait for ticket completion;
- dropping the accepted ticket after converting submission to public `accepted` must not cancel the already queued cycle;
- snapshot and submission calls are bounded and side-effect free except for the single requested submission.

Attach this cloneable control handle optionally to `ServerAppState`. Dependency-free/test states may omit it. Production startup must attach the handle before building the router while retaining unique host ownership for graceful shutdown.

Disabled production mode must still expose an honest Disabled/Ready/DisabledInert/Unavailable status through a control handle; do not omit the handle merely because manual submission is unavailable.

## Status endpoint

`GET /v1/admin/worktree/status`:

1. Authenticate using existing Server token verification and the accepted API-P8 Admin requirement.
2. Read one passive Server snapshot only.
3. Map internal categories exactly to accepted API-P8 DTO values.
4. Preserve:
   - Disabled → Ready / DisabledInert / Unavailable;
   - Running + Busy → Ready / Running / Busy;
   - Failed → NotReady / Failed / Failed;
   - counters and watcher hints as informational fields only.
5. Use the accepted checked watcher-hint conversion helper; impossible platform conversion maps to a fixed safe internal response without exposing the value.
6. Return HTTP 200 with `WorktreeStatusResponse` on success.
7. If runtime control is genuinely absent, return HTTP 503 with a fixed sanitized `ErrorResponse`; do not fabricate mode/lifecycle values.
8. Perform no DB query, filesystem access, provider call, runtime probe, submission or wait.

## Sync-once endpoint

`POST /v1/admin/worktree/sync-once`:

1. Accept the strict empty JSON object defined by `WorktreeSyncOnceRequest`.
2. Reject malformed/unknown body fields with HTTP 400 and a fixed sanitized `InvalidRequest` response; do not expose serde text or raw body content.
3. Authenticate through existing Server auth and validate the accepted API-P8 Admin requirement.
4. Do not expose or accept budget, path, mode override, force flag, ticket, generation or request id.
5. Server constructs the manual DryRun request from its already validated host config/action budget.
6. Use current authoritative snapshot/manual submission state without probe requests.
7. Map one bounded submission into exact API-P8 outcomes:
   - `Accepted` → `accepted`;
   - `Busy` → `busy`;
   - `NotStarted` → `not_started`;
   - `Cancelling` → `cancelling`;
   - `Shutdown` → `shutdown`;
   - mode/handle unavailable → `unavailable`;
   - failed host → `failed`.
8. A snapshot-to-submit race must be resolved by the authoritative submission result; never turn Busy/Cancelling/Shutdown into Accepted.
9. `accepted` means queued/submitted only. Drop or detach the internal ticket without polling it and without cancelling the queued cycle.
10. HTTP status mapping:
    - accepted → 202 Accepted;
    - busy → 409 Conflict;
    - not_started, cancelling, shutdown, unavailable → 503 Service Unavailable;
    - failed → 500 Internal Server Error.
11. Return the accepted `WorktreeSyncOnceResponse` body for all submission outcomes.
12. Missing token remains 401; invalid token remains 401; non-admin remains 403 using sanitized existing vocabulary.

## Tests

Add deterministic tests for:

- exact API-P8 fan-in blob identity or equivalent repository evidence;
- production-style state carries a cloneable control handle while the host owner remains uniquely shutdown-capable;
- Disabled status returns 200 with exact disabled/ready/inert/unavailable JSON;
- Running idle DryRun returns Available;
- Running Busy remains readiness `ready`;
- Failed status maps to not-ready/failed without raw error;
- counters and watcher hints survive mapping and do not change readiness;
- missing runtime control returns fixed safe 503 for status;
- GET status requires Admin auth;
- POST strict `{}` request accepts Admin and rejects unknown fields safely;
- non-admin and missing/invalid token mappings;
- exact HTTP and JSON mapping for every sync-once outcome;
- accepted response is immediate and does not poll/wait for completion;
- accepted ticket drop does not cancel the queued cycle and gate eventually clears through the existing owner path;
- Busy/no-overlap remains Worktree-owned;
- snapshot/submission race cannot produce false Accepted;
- no route response contains raw paths, roots, fingerprints, DB URLs, provider payloads, raw errors, tokens/hashes, cursors, idempotency values, request bodies, ticket or generation identifiers;
- existing SRV-P7B5, admin, auth, startup and graceful-shutdown tests remain green.

## Allowed Server paths

- `crates/haze-sync-server/src/worktree_host.rs`;
- `crates/haze-sync-server/src/worktree_status.rs`;
- `crates/haze-sync-server/src/state.rs`;
- `crates/haze-sync-server/src/main.rs`;
- `crates/haze-sync-server/src/routes/admin.rs`;
- focused admin/router/host tests;
- minimal module exports directly required;
- accepted API-P8 product files listed above;
- Server docs/control report when directly required.

## Forbidden scope

- semantic edits to accepted API-P8 product code;
- Worktree gate/runtime/watcher/executor changes;
- Storage/Core/CLI/Deployment product changes;
- migrations or workflow changes;
- new background task, poller, retry loop or completion watcher;
- broad app-state redesign;
- DB/provider/filesystem work in the new routes;
- exposing internal failures or secret-bearing fields;
- unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip. Obtain authoritative DB-capable Component CI on the exact final Server code-bearing SHA.

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-API-P8-WORKTREE-HTTP-FAN-IN`;
- `chat_name: server — W1 API-P8 Worktree HTTP Fan-In`;
- honest status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record:

- exact API blob verification;
- control-handle ownership design;
- endpoint/auth/body/HTTP mappings;
- evidence that submission does not wait and ticket drop does not cancel;
- changed paths;
- final SHA;
- exact DB-capable CI run and all step conclusions.

Do not claim CLEAN_ACCEPT, begin CLI-P6A, merge, or change PR draft state. A focused Server functional review follows.
