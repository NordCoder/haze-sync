# Integration, testing, and rollout

## Development model

Haze Sync is developed as a component-bounded system.

Each component has a long-lived `component/*` branch and local documentation:

```text
docs/component-contract.md
docs/implementation-plan.md
docs/implementation-log.md
docs/dependency-map.md
docs/decisions.md
control/state.md
control/prompt.md
control/report.md
```

System-level documentation lives in repository-level `docs/`. Component-local documentation lives inside component directories.

## Planning model

Component implementation plans are written before broad implementation work.

Planning is sequential because each component plan can refine or depend on contracts clarified by earlier planning passes. Implementation is parallel because component workers operate against contracts, mocks, and local fakes rather than requiring sibling runtime implementations.

A component plan should divide the component into implementation phases. A phase is not a tiny file-level patch; it is a bounded component-local goal executed by one or more agent executions.

## Phase lifecycle

The normal lifecycle for one implementation phase is:

```text
implementation worker
  -> clean-code reviewer
  -> CI
  -> fixer loop if CI fails
  -> accepted phase
```

The clean-code pass is local to the component and should check correctness, edge cases, coupling, duplication, tests, contract compliance, and non-goals.

The fixer pass should fix the minimum failing CI cause. If CI failure requires a contract or sibling-component change, the phase must report a contract/dependency blocker rather than hide the problem.

## Component-parallel implementation

Parallel implementation is safe only when components depend on stable contracts rather than unfinished sibling code.

Allowed approach:

```text
component implements local behavior
component tests against local fakes/mocks
component exposes public contract surface
integration phase wires real sibling components later
```

Unsafe approach:

```text
component imports sibling internals
component duplicates sibling policy
component edits shared route registration or CI outside its scope
component relies on provider secrets or live infrastructure for normal tests
```

## Integration surfaces

The following surfaces should be owned by dedicated integration/fan-in work, not hidden inside unrelated component phases:

- route registration across modules;
- public crate exports/barrels;
- shared CI workflow changes;
- database schema compatibility after consumers exist;
- runtime composition across Server + Storage + Core + API;
- Server + Worktree runtime wiring;
- adapter mode enforcement across Common/API/Server/CLI;
- GDrive adapter runtime connection to Core/API and mapping persistence;
- Obsidian plugin compatibility with live API responses;
- deployment Compose wiring;
- cross-component E2E tests;
- operational runbook consistency.

## Planning order for component docs

Recommended order for writing or normalizing component docs and plans:

```text
1. common
2. core
3. api
4. storage
5. server
6. worktree
7. obsidian-plugin
8. gdrive-adapter
9. cli
10. deployment
11. github-ci
12. docs-process
13. cross-component integration/e2e plan
```

This is an order for planning, not necessarily an implementation serialization order.

Rationale:

```text
common/core/api/storage/server
  define the shared contract center

worktree/obsidian/gdrive
  depend on stable sync/API semantics and adapter boundaries

cli/deployment/github-ci
  depend on runtime topology and operator surface decisions

docs-process/integration plan
  reflects the resulting component model
```

## Product integration stages

### Stage 1 — Server-side contract foundation

Goal:

```text
Core/API/Storage/Server agree on paths, hashes, revisions, idempotency,
operation log, safe errors, file PUT/GET, changes, conflict, delete,
admin/status, and doctor surfaces.
```

Gate:

```text
server route surfaces compile
Core safety tests pass
storage repositories and migrations are compatible
safe placeholder behavior is explicit where runtime dependencies are absent
```

### Stage 2 — Adapter-local foundations

Goal:

```text
Worktree, Obsidian plugin, and GDrive adapter each build local modules
against contracts and mocks without requiring full runtime integration.
```

Gate:

```text
worktree scanner/writer tests pass
plugin typecheck/build and pure tests pass
gdrive adapter mock tests pass
no adapter duplicates Core overwrite/conflict/delete policy
```

### Stage 3 — Local product slice

Goal:

```text
Core <-> Worktree <-> plugin-like client works in a local/mocked environment.
```

Gate:

```text
client can PUT file
Core stores revision
worktree materializes file
worktree edit returns to Core
changes API shows update
same-content duplicate is safe
stale write creates conflict_saved
```

### Stage 4 — Google Drive import/export slice

Goal:

```text
GDrive adapter can dry-run, import supported files, export Core changes,
maintain mapping, and avoid echo loops in mock or isolated test-folder setups.
```

Gate:

```text
gdrive dry-run reports supported/ignored/duplicate/potential-delete counts
import_only submits files to Core safely
export_only writes Core changes to Drive-like provider
own writes are ignored as echo
provider mass delete is blocked
```

### Stage 5 — Near-realtime behavior

Goal:

```text
Watchers/webhooks improve latency while scans/reconciliation preserve correctness.
```

Gate:

```text
Obsidian event push works while app is open
periodic pull observes remote changes
GDrive changes feed and webhook enqueue work
fallback polling works
invalid cursor triggers safe reconciliation
```

### Stage 6 — Release-candidate hardening

Goal:

```text
End-to-end sync, failure injection, doctor, deployment, backup, restore,
and rollout docs are coherent enough for staged real-vault validation.
```

Gate:

```text
E2E suites pass
doctor passes after E2E
failure injection preserves data
bootstrap sequence is command-supported or precisely documented
rollback is documented
no CI check needs production secrets
```

## Test layers

Use these layers together:

```text
unit tests
component integration tests
server route tests
adapter mock tests
local temp-dir filesystem tests
real Postgres test-support where scoped
mock provider E2E tests
failure injection tests
manual bootstrap validation
```

Unit tests prove local semantics. E2E tests prove product coherence. CI green on components is necessary but not sufficient for release readiness.

## Required safety tests

Core/server/storage tests must cover:

- invalid path rejection;
- hash verification;
- base=current accepted update;
- base=old different content creates conflict;
- base=null existing different content creates conflict;
- same-content duplicate ignored;
- idempotency same-key same-request behavior;
- idempotency same-key different-request conflict;
- delete creates tombstone, not hard delete;
- mass delete guard blocks unsafe bursts;
- concurrent writes do not corrupt state.

Adapter tests must cover:

- local/provider scan reports;
- ignored paths/file classes;
- echo guard;
- cursor advancement after success only;
- dry-run mode no mutation;
- import/export mode enforcement;
- delete candidates and mass-delete stop behavior.

## Failure injection

Before V1 release candidate, test or manually validate:

- crash after blob write before DB commit;
- crash after DB commit before adapter export;
- network failure during plugin upload;
- provider export succeeds but metadata read fails;
- duplicate idempotency key;
- concurrent writes to same path;
- disk full or permission denied in worktree;
- invalid GDrive changes cursor;
- dangerous provider delete burst.

## Bootstrap sequence

The bootstrap sequence must start from exactly one trusted source.

Current intended trusted starting source:

```text
Google Drive vault
```

Safe sequence:

```text
1. backup current Google Drive/VPS state
2. deploy Core server and database
3. keep adapters disabled/dry-run initially
4. GDrive dry-run scan
5. GDrive import-only into Core
6. verify Core status and doctor
7. materialize Worktree export-only
8. verify Worktree dry-run/doctor
9. Obsidian initial pull-only into empty/local vault
10. enable bidirectional one adapter at a time
11. run smoke checks after every enablement step
```

Do not enable multiple bidirectional adapters at once during initial rollout.

## Rollback principles

Before bidirectional mode:

```text
stop new adapters
preserve Core DB/object store for diagnosis
continue original Google Drive workflow if needed
```

After bidirectional mode:

```text
stop GDrive adapter first
stop Worktree runtime
turn off Obsidian auto-sync
export Core state to separate backup
compare states before restoring anything
```

Rollback must preserve evidence. Do not repair by deleting metadata or provider files blindly.

## Operational readiness

Before real-vault rollout, the system must have:

- backups or backup commands/checklists;
- status command;
- adapters list/mode visibility;
- doctor checks for DB, object store, blobs, cursors, mappings, worktree drift, and token sanity where implemented;
- dry-run scans for Google Drive and Worktree;
- delete guard configuration;
- safe logs without secrets;
- deployment docs for directories, env vars, and healthchecks;
- documented emergency stop.

## CI expectations

CI should remain secret-free and provider-free by default.

Expected CI categories:

- Rust formatting/check/test/clippy;
- TypeScript plugin typecheck/build/tests when present;
- Docker Compose config validation;
- component branch status visibility;
- E2E/mock tests when added;
- no production OAuth, provider tokens, deployment credentials, or live vault access.
