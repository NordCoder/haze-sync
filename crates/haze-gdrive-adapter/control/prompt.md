# W1-GDA-GDA-P4-LONG-RUNNING-RUNTIME

## Routing envelope

- repository: `NordCoder/haze-sync`
- component: `gdrive-adapter`
- component_path: `crates/haze-gdrive-adapter`
- role: `implementation-worker`
- agent_execution_id: `gdrive-adapter-GDA-GDA-P4-impl-20260717102652-509ee26f`
- branch: `component/gdrive-adapter`
- pull_request: `#50`
- wave: `W1`
- phase: `GDA-GDA-P4-LONG-RUNNING-RUNTIME`
- control_prompt_path: `crates/haze-gdrive-adapter/control/prompt.md`
- control_report_path: `crates/haze-gdrive-adapter/control/report.md`
- expected_report_type: `IMPLEMENTATION`

This component has one dedicated execution chat for implementation, clean-code-review, and fixer work. For this execution only, act as `implementation-worker`. Do not derive routing or scope from prior chat messages.

## Mandatory source order

Read and apply, in order:

1. the current project implementation manifest and report template;
2. `crates/haze-gdrive-adapter/docs/component-contract.md`;
3. `crates/haze-gdrive-adapter/docs/implementation-plan.md`;
4. `crates/haze-gdrive-adapter/docs/implementation-log.md`;
5. `crates/haze-gdrive-adapter/docs/dependency-map.md`;
6. `crates/haze-gdrive-adapter/docs/decisions.md`;
7. architecture review blob `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
8. this prompt;
9. current branch code, tests, manifests, lockfile and PR metadata.

Old control files are read-only evidence unless this prompt identifies them. Do not archive control files.

## Accepted inputs

The following are accepted and must remain intact unless a proven component-local defect requires a compatible correction:

- GDrive P2 HTTP/durable-state candidate: `746dc8790643e13e85553ff94f6b124a5c686127`;
- GDrive P2 final clean-review report blob: `e78d1d5141c17cd60f0487d251cf73a8c985e6de` (`CLEAN_ACCEPT`);
- Component CI run `29539680811`, run number `2060`, success on that exact SHA;
- Cargo-generated lockfile blob: `882be8e8ce61ac4c77e8bdaec45d1cbaa030aa86`;
- accepted OAuth/auth candidate: `7f00a60641ca157907d0e75e4ab1bb47c05f03c9`;
- accepted API GDrive contract: `c60c3976696da1970d539e5cff6e9f74a61fc10e`;
- accepted Server GDrive routes/transactions: `c023b83e1e6f502e7d2261acccb871dd5588edf1`;
- accepted Storage durable-state contract: `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`.

Fixed architecture:

- one standalone long-running `haze-gdrive-adapter` process per configured adapter identity/root;
- public authenticated HTTP through accepted API/Server contracts;
- durable mapping/cursor/echo/delete state owned by Storage and reached only through Server/API;
- no adapter database access;
- one serialized authoritative scheduler per adapter identity;
- full scan is the correctness backstop; change feed is a latency optimization;
- Core remains authoritative for revision, conflict and delete policy.

## Objective

Replace the current immediate-exit lifecycle skeleton with a real, bounded, standalone long-running runtime that composes the already implemented scan, change-feed, export, delete-guard, OAuth/auth and durable-state boundaries.

Do not deliver another fake-only or immediate-exit skeleton. Implement all missing concrete GDrive-local adapters needed for the standalone process to perform its accepted responsibilities. If a required public API/Server contract is genuinely absent or incompatible, report `BLOCKED_BY_CONTRACT` instead of inventing a sibling-owned contract.

## Required implementation

### 1. Concrete process composition

The binary/runtime must:

- load and validate config and explicit adapter identity;
- load the read-only OAuth credential file safely;
- construct a concrete bounded token endpoint and Google Drive provider client where the current fake-first traits lack production implementations;
- construct the accepted bounded HTTP durable-state client;
- construct concrete GDrive-local HTTP gateways for existing public Haze Server file/change/server-info routes when required by the existing import/export/delete modules;
- load the bounded adapter-private durable-state snapshot before processing;
- wire existing full-scan, change-feed, export, echo and delete-guard modules without duplicating their policy;
- remain alive until cooperative shutdown or a startup-fatal condition.

Do not infer adapter identity from credentials or provider metadata. Do not access PostgreSQL or Storage internals.

### 2. Serialized scheduler

Implement one authoritative scheduler with no overlapping state-transition cycles:

- bounded change-feed poll cadence from config;
- bounded periodic full-scan cadence from config;
- full scan cannot be postponed indefinitely by polling, retries or jitter;
- bounded exponential backoff with bounded jitter for retryable provider/Server failures;
- cursor invalidation, permission uncertainty or mapping inconsistency schedules a full scan before cursor reinitialization;
- one cycle owns checkpoint/cursor progression; bounded internal concurrency is allowed only where it cannot reorder durable progress.

Use injectable clock/sleeper/cancellation/random-jitter boundaries so tests are deterministic and do not depend on long wall-clock sleeps.

### 3. Mode enforcement

Enforce the accepted capability matrix before provider, Core or durable-state work:

- `disabled`: lifecycle/local safe status only; no provider/Core reads and no durable mutation;
- `dry_run` and `read_only`: observation/planning reads only; no Core writes, provider writes/trash, cursor/checkpoint advancement or mapping/echo/delete mutation;
- `import_only`: provider reads, accepted Core writes and permitted durable import/cursor/delete-candidate commits; no provider writes/trash;
- `export_only`: Core reads, permitted provider writes/trash and durable mapping/echo/export checkpoint commits; no provider-originated Core writes;
- `bidirectional`: both directions, preserving every accepted guard.

A mode denial must occur before constructing or sending the forbidden mutation.

### 4. Durable progress and crash safety

Preserve these rules:

- Drive cursor advances only after the complete represented work and corresponding durable commit succeed;
- Core export checkpoint advances only after provider confirmation and mapping/echo commit succeed;
- mapping facts are committed only after confirmed Core/provider outcomes;
- compare-and-commit POST is never blindly or automatically retried;
- after stale/conflict/ambiguous outcomes, refetch/reconcile before any new commit attempt;
- deterministic idempotency and operation identities allow Core mutation replay without duplicate mutation;
- provider mutations without provider idempotency use preconditions and post-crash reconciliation before repetition;
- no unresolved item may be skipped by a cursor or checkpoint;
- no unbounded in-memory backlog is created while Server is unavailable.

### 5. Failure and degraded-state behavior

- invalid config or corrupt credential file: fail startup closed;
- revoked credentials, insufficient scope, inaccessible root or mass-delete block: process remains alive in a safe degraded/blocked state and performs no mutations;
- retryable provider rate limit/unavailability or Server outage: remain alive, preserve durable progress and retry with bounded backoff;
- invalid cursor: preserve durable state and schedule a correctness full scan;
- invariant/state-version mismatch: stop affected processing fail-closed and expose only a safe category;
- never include raw dependency errors, response bodies or private facts in operator-safe output.

This phase may implement safe local runtime state required for lifecycle/testing. Do not create the deferred public status/control API contract.

### 6. Graceful shutdown

Support cooperative process shutdown, including normal termination signals where the platform permits:

- stop accepting new cycles;
- cancel waits promptly;
- allow only the current bounded atomic HTTP/provider operation to finish;
- commit only complete outcomes;
- enforce a bounded shutdown deadline;
- exit without advancing unresolved cursor/checkpoint state;
- leave restart reconciliation able to resolve ambiguity safely.

### 7. Tests

Add deterministic tests covering at least:

- binary/runtime remains active rather than immediately exiting;
- startup construction and observation-only preflight;
- exactly one authoritative cycle and no overlap;
- poll/full-scan scheduling and full-scan deadline fairness;
- all adapter modes with zero forbidden calls/mutations;
- transient provider and Server backoff without progress loss;
- auth revoked/scope loss/root unreachable degraded behavior;
- cursor invalidation triggering full scan;
- stale durable-state conflict requiring refetch rather than blind POST retry;
- crash boundaries before and after Core/provider mutation and before state commit;
- graceful shutdown during wait and during bounded work;
- comprehensive redaction sentinels.

Ordinary CI must use fakes, synthetic fixtures and loopback-only transport where necessary. It must not use real Google credentials, a live Google endpoint, a live Server, production data or unrestricted external network.

### 8. Documentation

Update the GDrive implementation log and focused runtime documentation to describe the implemented lifecycle, recovery boundaries, test model and remaining later-phase gates. Do not claim deployment readiness.

## Allowed scope

- `crates/haze-gdrive-adapter/src/**`;
- focused GDrive tests/fixtures;
- `crates/haze-gdrive-adapter/Cargo.toml` and repository `Cargo.lock` only for minimal runtime/network/signal dependencies actually required;
- `crates/haze-gdrive-adapter/docs/**`;
- exact existing API fan-in files are read-only and must remain byte-identical;
- `crates/haze-gdrive-adapter/control/report.md`.

The component scope is not limited to one file. A broader internal GDrive refactor is allowed when required for a clean runtime composition and must be explained in the report.

## Forbidden scope

- semantic edits to API, Server, Storage, Core, Common, Worktree, Obsidian, CLI or Deployment;
- direct database access or `DATABASE_URL` consumption;
- new public status/control DTOs or routes;
- operator CLI commands or Deployment service packaging;
- hard delete or bypass of Core/delete-candidate policy;
- Server-hosted or dual-hosted provider loops;
- automatic non-idempotent POST retry;
- real credentials, tokens, provider payload archives or external-network CI;
- workflow edits, test weakening, merge, rebase, force-push or PR draft-state changes.

## CI and commit honesty

- Product, test, manifest, dependency and documentation commits must not use CI skip.
- Obtain a real final code-bearing SHA and full Component CI for that exact SHA.
- Record fmt/check/test/clippy and diagnostics-finalizer conclusions separately.
- A skipped workflow is not green CI.
- If CI is pending, report `SELF_ACCEPT_PENDING_CI` with exact pending coordinates.
- If CI is red, do not diagnose from memory and do not claim success; record the exact run/artifact metadata for a later fixer.
- Do not fabricate commit, blob, job, artifact or run identifiers.

## Reporting

Write only `crates/haze-gdrive-adapter/control/report.md` using the project report template.

The report routing envelope must include exactly:

- `REPORT_TYPE: IMPLEMENTATION`;
- terminal `STATUS` from the implementation vocabulary;
- `role: implementation-worker`;
- `agent_execution_id: gdrive-adapter-GDA-GDA-P4-impl-20260717102652-509ee26f`;
- component `gdrive-adapter`;
- branch `component/gdrive-adapter`;
- wave `W1`;
- phase_id `GDA-GDA-P4-LONG-RUNNING-RUNTIME`;
- control prompt/report paths;
- prompt commit SHA and prompt blob SHA supplied by the dispatcher launch envelope.

Also record changed files, runtime composition, mode matrix evidence, scheduler/recovery/shutdown behavior, concrete client boundaries, tests, secrecy review, final code-bearing SHA and exact CI metadata. Do not archive prompt/report files and do not claim `CLEAN_ACCEPT`, deployment readiness or merge readiness.

## Next gate

- `SELF_ACCEPT` with exact-SHA green CI -> focused GDrive long-running runtime clean-code/security review;
- `SELF_ACCEPT_PENDING_CI` -> Orchestrator verifies CI before transition;
- `SELF_NEEDS_FIX` -> continuation or focused component fixer;
- contract/dependency/tooling blocker -> stop safely with exact evidence.
