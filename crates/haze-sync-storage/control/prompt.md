# W1-STOR-GDA-P1-DURABLE-STATE

Before starting, name this worker chat exactly:

`storage — W1 STOR-GDA-P11 GDrive Durable State`

## Authoritative repository and control slot

```text
repository: NordCoder/haze-sync
component: storage
component path: crates/haze-sync-storage
branch/ref to read and write: component/storage
PR: #47
base branch: main
role: implementation-worker
phase: STOR-GDA-P1-DURABLE-STATE
```

`NordCoder/haze-sync` is the correct current repository. Repository access is verified. There is no migrated-repository lookup or migration audit in this phase.

Read `control/state.md` and `control/prompt.md` explicitly from `component/storage`. Do not use the default-branch control files, project handoff text, cached snippets, or historical `ACCEPTED_HOLD` files as the active slot.

The authoritative active state is:

```text
status: PROMPT_READY
active_agent_role: implementation-worker
assigned_chat_name: storage — W1 STOR-GDA-P11 GDrive Durable State
phase: STOR-GDA-P1-DURABLE-STATE
```

If a different state is observed, first re-fetch the exact files from `ref=component/storage`. Do not replace this task with a migrated-repository audit.

Fetch the actual branch head before editing because Orchestrator control-only commits follow the earlier observed head.

Do not merge, change draft state, rewrite history, rebase, force-push, modify sibling branches, or perform unrelated cleanup.

## Accepted baseline

- previous accepted Storage code-bearing SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- PR #47 is open, draft, mergeable and unmerged;
- GDrive fan-in architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
- GDrive synchronized baseline: `f9a2da6eb9ac6f59b1ec18ae4d85eb51964f3cbe`.

## Fixed architecture

Implement these decisions; do not redesign them:

1. Storage owns durable GDrive runtime state and schema.
2. The standalone GDrive Adapter never receives `DATABASE_URL` and never accesses Storage/SQLx directly.
3. Server will later own transaction and application choreography over passive Storage repositories.
4. API/Server transport and public DTOs are out of scope here.
5. Core remains authoritative for conflict, revision, delete/tombstone and idempotency policy.
6. Storage persists caller-decided facts; it does not implement provider or Core policy.
7. Cursor/checkpoint advancement is allowed only after complete caller-confirmed outcomes.
8. Mapping, echo, delete-candidate and operation state require compare-and-commit/versioned transaction semantics.
9. Secret values, OAuth data, bearer values, raw provider payloads and public status vocabulary are never stored or exposed by this phase.

## Required durable facts

Provide a versioned, adapter-scoped persistence boundary sufficient for later Server/API-mediated restart and replay:

- stable adapter identity;
- persisted state version used for compare-and-commit;
- Drive change cursor and cursor generation/version metadata;
- Core export sequence checkpoint;
- Drive file ↔ Core path/object/revision mapping facts;
- provider version/head revision/checksum/modified-time facts required for reconciliation;
- echo confirmation state for adapter-originated provider writes;
- delete candidate first-seen, last-seen, confirmation generation and blocked state;
- delete confirmation/audit references, without defining admin unlock policy;
- last successful import/export/provider-mutation operation identifiers;
- retry-safe operation/idempotency records or leases where required for deterministic replay.

Use bounded typed models. Raw provider cursor data may be persisted internally only where correctness requires it, but Debug/Display/errors/tests/reports must redact or summarize it.

## Repository and transaction contracts

Add or extend passive Storage repositories so a later Server caller can execute one caller-owned PostgreSQL transaction.

Required invariants:

- repository functions accept a caller-owned transaction/connection where atomic composition is required;
- Storage must not begin or commit hidden transactions inside compare-and-commit operations;
- snapshot reads are bounded and deterministic;
- expected persisted version is mandatory for mutation;
- stale version fails without partial mutation;
- Drive cursor cannot regress;
- cursor gap/generation mismatch fails safely unless the exact accepted transition permits it;
- cursor never advances past unresolved work;
- Core export checkpoint cannot regress;
- mapping is updated only from caller-confirmed Core/provider outcomes;
- mapping update and echo confirmation can be committed atomically;
- delete-candidate validation and state transition can be committed atomically;
- duplicate operation identifiers replay the same stored outcome or fail as an idempotency conflict when payload/facts differ;
- rollback leaves mapping, cursor, checkpoint, echo, candidate and operation facts unchanged;
- no repository method performs Google calls, Server calls, Core policy or provider mutation.

Preserve all accepted STOR-P10 caller-owned transaction, locking, redaction and migration-safety behavior.

## Schema and migration

Add the minimum contiguous migration after the current migration sequence, expected to be `0011` unless the actual branch contains a later migration at worker start.

The migration must:

- support fresh databases;
- support the accepted current/pre-state migration path;
- avoid destructive reset/drop/down behavior;
- use explicit keys, uniqueness and foreign-key boundaries appropriate for adapter-scoped state;
- support optimistic versioning and deterministic operation replay;
- avoid storing OAuth tokens, bearer tokens, raw provider payloads or file contents;
- fail safely on incompatible pre-existing structures rather than silently deleting or rewriting data.

Do not change an already accepted migration unless a concrete migration defect makes the new migration impossible. Report such a case as `BLOCKED_BY_CONTRACT` instead of rewriting accepted history.

## Allowed product scope

- `crates/haze-sync-storage/src/models/**`;
- `crates/haze-sync-storage/src/repositories/**`;
- `crates/haze-sync-storage/src/schema/**`;
- one minimum new file under `migrations/**`;
- focused Storage PostgreSQL/unit tests;
- focused `crates/haze-sync-storage/docs/**` updates;
- `crates/haze-sync-storage/control/report.md`.

A focused repository module such as `gdrive_state.rs` is allowed. Extend `gdrive_mapping.rs` only if its single responsibility remains clear.

## Required tests

Add unit and real PostgreSQL evidence for at least:

1. fresh migration creates the durable-state surface;
2. migration from accepted current pre-state succeeds without data loss;
3. incompatible pre-existing state fails before destructive mutation;
4. empty initial adapter snapshot;
5. bounded deterministic snapshot containing mapping/cursor/checkpoint/candidate facts;
6. successful expected-version compare-and-commit;
7. stale expected version rejects every mutation atomically;
8. cursor regression rejects atomically;
9. cursor generation/gap mismatch rejects safely;
10. Core export checkpoint regression rejects atomically;
11. mapping plus echo state commits atomically;
12. delete-candidate validation plus transition commits atomically;
13. transaction rollback preserves prior state;
14. duplicate operation replay is deterministic;
15. same operation identity with different facts conflicts;
16. adapter identity isolation;
17. Debug/Display/errors reveal no raw cursor, Drive payload, secret, database URL or sensitive absolute path;
18. existing Storage tests, including STOR-P10 strict PostgreSQL evidence, remain green.

Use synthetic identifiers and provider facts only. No live Google credentials or provider calls.

## Documentation

Update Storage documentation minimally to record:

- Storage ownership of GDrive durable state;
- Server-owned transaction choreography;
- no direct Adapter database access;
- stored facts versus forbidden secrets/provider payloads;
- compare-and-commit and crash-replay invariants;
- schema/migration addition;
- exact downstream fan-in dependency for API and Server.

Do not redesign GDrive runtime, API routes, OAuth, scheduling or Deployment in Storage docs.

## Forbidden

- API DTO or route changes;
- Server application/route changes;
- GDrive Adapter product changes;
- direct database access from Adapter;
- Google/OAuth implementation;
- scheduling or background jobs;
- Deployment or Compose changes;
- Core policy or delete-unlock policy;
- admin/operator controls;
- real secrets, credentials, URLs, dumps or provider payloads;
- workflow changes;
- broad repository refactors;
- weakening existing migration or transaction tests.

## Validation and CI

Create real product/migration commits without CI skip.

Required evidence on the exact final code-bearing SHA:

- `cargo fmt --all --check`;
- Storage check/test/clippy through Component CI;
- mandatory DB-capable Storage PostgreSQL verification;
- fresh and accepted pre-state migration evidence;
- compare-and-commit rollback/replay evidence;
- PR #47 remains open, draft and unmerged.

If DB tooling or authoritative CI cannot run, use `BLOCKED_BY_TOOLING`; do not claim acceptance from unit tests alone.

## Report

Write only `crates/haze-sync-storage/control/report.md` using the project report template.

Set:

```text
REPORT_TYPE: IMPLEMENTATION
phase_id: STOR-GDA-P1-DURABLE-STATE
chat_name: storage — W1 STOR-GDA-P11 GDrive Durable State
```

Allowed statuses:

- `SELF_ACCEPT`;
- `SELF_ACCEPT_PENDING_CI`;
- `SELF_NEEDS_FIX`;
- `BLOCKED_BY_CONTRACT`;
- `BLOCKED_BY_DEPENDENCY`;
- `BLOCKED_BY_TOOLING`.

Do not invent `BLOCKED_BY_MIGRATED_REPOSITORY_ACCESS`: repository access and coordinates are already verified for this slot.

Record exact changed paths, migration number/schema, repository contracts, transaction/version/cursor/idempotency invariants, PostgreSQL evidence, secrecy evidence, final code-bearing SHA, exact CI, and downstream contracts unblocked.

Do not claim `CLEAN_ACCEPT` or merge readiness. A focused Storage clean/DB review follows.
