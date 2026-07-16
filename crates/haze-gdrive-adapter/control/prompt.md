# W1-GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 GDA-GDA-P2 HTTP Durable State Client`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: implementation-worker
Phase: `GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT`

This is the only active GDrive phase. Do not begin the long-running runtime, provider synchronization, status/control or Deployment phases.

## Accepted inputs

- accepted config/mode SHA: `fcc04afd1fe9808656d9bc2effbfff7160efe9fc`;
- config clean-review blob: `d9bc16c1a50755ccaecd1b51add231bf52e7e35f`;
- accepted OAuth/auth SHA: `7f00a60641ca157907d0e75e4ab1bb47c05f03c9`;
- OAuth clean-review blob: `27a465aabd66975f2c519516d8086292639d5cd0`;
- accepted API GDrive contract SHA: `c60c3976696da1970d539e5cff6e9f74a61fc10e`;
- API clean-review blob: `55ff6047c9c6c0f6f548f10197b76706c0a244e1`;
- accepted Server GDrive routes SHA: `c023b83e1e6f502e7d2261acccb871dd5588edf1`;
- Server clean-review blob: `1c223ebda33a1550fa8cbf38a15079ef7810c63d`;
- Server DB-capable CI run: `29494321838`, run number `2045`, success.

Accepted routes:
- `GET /v1/adapters/{adapter_id}/gdrive/state`;
- `POST /v1/adapters/{adapter_id}/gdrive/state/commit`.

## Fixed architecture

1. GDrive Adapter consumes public Server/API HTTP contracts only. It never accesses Storage or PostgreSQL directly.
2. Server authenticates the bearer adapter token and verifies that its principal identity matches `{adapter_id}`.
3. Therefore adapter identity must be explicit configuration. Do not infer identity from the token or provider metadata.
4. API owns DTO/header/error vocabulary; Server owns route execution and transactions; this component owns HTTP transport, safe decoding, mode-aware call gating and client-side retry classification.
5. This phase does not implement polling, reconciliation, Google operations, import/export planning or automatic retries.

## Required deliverables

1. Add required bounded config `HAZE_GDRIVE_ADAPTER_ID`:
   - parse with the accepted `AdapterId` rules;
   - store as a typed value or validated opaque wrapper;
   - reject missing/invalid values at startup;
   - do not expose it together with token/request details in Debug or errors;
   - update synthetic config tests and docs.
2. Consume the exact accepted API GDrive DTO, route and header contracts.
   - The GDrive branch currently lacks the accepted API GDrive files.
   - Exact byte-identical fan-in of the minimum required accepted API files from SHA `c60c3976696da1970d539e5cff6e9f74a61fc10e` is allowed if needed.
   - Record every fanned-in path and blob identity.
   - Do not semantically edit API owner files; report `BLOCKED_BY_CONTRACT` if exact reuse is impossible.
3. Define a fakeable durable-state client abstraction and a concrete HTTP implementation.
4. Implement private state GET:
   - bearer authentication from the redacted adapter token;
   - exact adapter-id route path;
   - bounded `after_path` and `limit` query support;
   - strict decoding of the accepted private snapshot;
   - bounded pagination/collection with loop and item-count protection where collection is offered;
   - no Admin surface consumption.
5. Implement compare-and-commit POST:
   - exact accepted commit DTO;
   - caller-supplied mandatory `Idempotency-Key` using the accepted header type;
   - exact route path, bearer authentication and JSON encoding;
   - strict decoding of committed/replayed/stale/gap/regression/mapping/idempotency/validation outcomes and route-error envelopes;
   - never derive an idempotency key from secrets or silently replace the caller value.
6. Add a mode-aware facade:
   - disabled mode performs no HTTP requests;
   - state reads require accepted `core_reads` capability;
   - commit requires `durable_state_mutation` capability;
   - denied calls fail locally with a safe typed category.
7. Implement safe transport behavior:
   - bounded connect/request timeout;
   - redirects disabled;
   - bounded response body before JSON decoding;
   - no automatic POST retry in this phase;
   - typed retryability for transport unavailable, HTTP 503 and safe internal failures versus non-retryable auth/forbidden/not-found/validation/conflict categories;
   - malformed, oversized or unexpected responses fail closed.
8. Redact all client/request/error surfaces:
   - bearer token, Idempotency-Key, request body, private cursor, paths, provider facts and response body never appear in Debug/Display/errors/log-ready strings;
   - do not include raw URL paths containing adapter identity in failure text;
   - safe fixed categories only.
9. Add deterministic tests with an in-process fake HTTP server or transport fake:
   - exact GET path/query/auth and private snapshot decoding;
   - bounded pagination and repeated-cursor/next-path protection;
   - exact POST path/auth/content type/idempotency and all accepted success/outcome categories;
   - invalid_cursor_state and safe internal envelopes;
   - 401/403/404/409/422/500/503 classification;
   - malformed/unknown/oversized JSON and timeout/transport failure;
   - redirect refusal;
   - mode-denied requests produce zero transport calls;
   - secrecy sentinels prove no token, idempotency, cursor, path, provider fact or body leakage.
10. Update GDrive implementation log and relevant docs minimally.

## Allowed scope

- `crates/haze-gdrive-adapter/src/**`;
- `crates/haze-gdrive-adapter/Cargo.toml` and lockfile for minimum HTTP/JSON/async dependencies;
- focused GDrive tests and synthetic fixtures;
- GDrive docs and control report;
- minimum exact byte-identical accepted API dependency fan-in and module wiring only where required.

## Forbidden

- API semantic changes or Server/Storage/Core product edits;
- direct DB access;
- Google Drive synchronization or provider mutations;
- scheduler, polling loop or long-running runtime composition;
- automatic commit retries;
- public status/operator-control contracts;
- CLI or Deployment work;
- real tokens, credentials, provider access or external-network CI;
- raw secrets/private state in logs or errors;
- sibling/workflow changes beyond the explicitly allowed exact API dependency fan-in;
- test weakening, merge, rebase, force-push or PR draft-state changes.

Create product/test changes without CI skip. Obtain full exact-SHA Component CI with fmt/check/test/clippy and diagnostics finalization green. Ordinary CI must use only synthetic/fake HTTP and no live Server or Google credentials.

Write only `crates/haze-gdrive-adapter/control/report.md` with:
- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT`;
- `chat_name: gdrive-adapter — W1 GDA-GDA-P2 HTTP Durable State Client`;
- status `SELF_ACCEPT`, `SELF_ACCEPT_PENDING_CI`, `SELF_NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_DEPENDENCY`, or `BLOCKED_BY_TOOLING`.

Record adapter-id config, exact API fan-in identity, client methods, mode gating, transport/error mapping, secrecy tests, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT, runtime readiness or deployment readiness.
