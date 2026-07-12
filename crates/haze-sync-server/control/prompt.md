# W1-SRV-P7B3-EXACT-SHA-FAN-IN — Integrate accepted Worktree and Storage snapshots

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B3 Exact-SHA Fan-In`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker
Phase: SRV-P7B3-EXACT-SHA-FAN-IN

This is an integration-preparation phase. Do not implement the real Worktree cycle executor yet.

## Why this phase exists

The three prerequisite owner phases are clean-accepted, but their exact product snapshots are not yet present in one integration branch:

- WT-P10 accepted Worktree SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- STOR-P10 accepted Storage SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- SRV-P7B2 accepted Server SHA: `647dce7b624d67663632808906896cb6745ea7e7`.

Live comparison proves `component/server` diverges from both accepted owner snapshots. SRV-P7B3 may begin only after exact owner product fan-in, green DB-capable CI and mandatory integration clean review.

## Authoritative evidence

Worktree:

- clean status: `CLEAN_ACCEPT`;
- accepted SHA: `1942946331e8362f19907ab6ad4eb779da70fd57`;
- CI run: `29167289593`, run number `1799`, success;
- all later Worktree branch commits are control-only.

Storage:

- clean status: `CLEAN_ACCEPT`;
- accepted SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- CI run: `29185466870`, run number `1833`, success;
- strict PostgreSQL verification and five mandatory evidence checks passed;
- cross-branch acceptance report commit: `13a0c80123061848235676b2a7dc7c3a3c644dee`.

Server:

- clean status: `CLEAN_ACCEPT`;
- accepted SHA: `647dce7b624d67663632808906896cb6745ea7e7`;
- CI run: `29186058268`, run number `1835`, success;
- clean report commit: `053eea1496bf9b541b462a82989b4cd956ed7276`;
- all later Server changes before this prompt are control-only.

## Required git method

Use an authenticated local git clone or isolated worktree. Work from the current `component/server` head.

Allowed history operations:

- fetch exact accepted commits;
- restore/check out explicitly scoped owner-owned product paths from the exact SHAs;
- create normal commits;
- push normally to `component/server`.

Forbidden:

- rebase;
- reset or history rewrite;
- force push;
- merging a whole component branch;
- merging into `main`;
- changing PR #45 draft/open state.

A whole-branch merge is forbidden because it would import sibling control history, stale copies of unrelated components and obsolete workflows.

## Exact product fan-in

Synchronize the complete non-control Worktree product snapshot from:

`1942946331e8362f19907ab6ad4eb779da70fd57`

This includes all accepted Worktree crate-owned product, test, fixture and documentation files, including at minimum:

- `crates/haze-sync-worktree/Cargo.toml`;
- `crates/haze-sync-worktree/src/**`;
- `crates/haze-sync-worktree/docs/**`;
- any other non-control Worktree-owned product/test/fixture files present at that exact SHA.

Synchronize the complete non-control Storage product snapshot from:

`66b6a1f554aae1d1b774cc88560d46dd140c7a54`

This includes all accepted Storage-owned migration, crate product, test, fixture and documentation files, including at minimum:

- `migrations/**`;
- `crates/haze-sync-storage/Cargo.toml`;
- `crates/haze-sync-storage/src/**`;
- `crates/haze-sync-storage/tests/**`;
- `crates/haze-sync-storage/docs/**`;
- any other non-control Storage-owned product/test/fixture files present at that exact SHA.

Explicitly exclude from owner snapshot transfer:

- every sibling `control/**` directory;
- sibling component reports/prompts/logs/state;
- `.github/workflows/component-ci.yml` from Worktree or Storage branches;
- any stale copy of Server, Core, API, Common, CLI, GDrive, Obsidian or Deployment files.

Preserve the current accepted Server Component CI workflow, including isolated PostgreSQL services for Server and Storage, complete workspace test coverage and diagnostics finalization.

Update `Cargo.lock` only when required by the synchronized accepted manifests and normal Cargo tooling.

## Mechanical compatibility corrections

Focused Server-local corrections are allowed only when required to compile or test against the exact accepted owner contracts.

Allowed examples:

- import/path/type adjustments caused directly by the accepted Worktree async contract;
- dependency or feature declarations required by the accepted Storage/Worktree snapshots;
- focused integration tests proving accepted interfaces compose.

Forbidden in this phase:

- implementing `ServerWorktreeCycleExecutor` behavior;
- import/delete/export cycle choreography;
- hosted scheduling or background tasks;
- public API/status DTO changes;
- runtime policy invention;
- route behavior changes;
- Core policy changes;
- schema redesign beyond exact accepted Storage files;
- broad refactoring.

If accepted contracts cannot compose without a new architectural decision, report `SELF_BLOCKED_BY_CONTRACT` rather than inventing behavior.

## Mandatory verification

Before committing, prove exact owner snapshot equality for every transferred non-control product path using git diff against the accepted SHA.

The report must list:

- exact Worktree pathspecs and equality result;
- exact Storage pathspecs and equality result;
- excluded control/workflow paths;
- every Server-local compatibility file changed and why;
- whether `Cargo.lock` changed and why.

Also verify:

- no Worktree/Storage control file was imported;
- no temporary write-enabled workflow exists;
- `crates/haze-sync-server/Cargo.toml` keeps Storage `test-support` out of normal dependencies and enables it only in dev/test scope;
- current Server application-service behavior remains unchanged;
- PR #45 remains open, draft, unmerged and mergeable.

## CI acceptance

A normal code-bearing Component CI run is mandatory.

It must pass:

- PostgreSQL Server service initialization;
- PostgreSQL Storage service initialization;
- cargo fmt;
- cargo check;
- isolated Server tests;
- isolated Storage tests, including strict STOR-P10 database evidence;
- all remaining workspace tests;
- cargo clippy with warnings denied;
- diagnostics finalizer.

Do not use a report-only commit as CI evidence.

If CI fails, do not guess from wrapper step summaries. Record the exact run/artifact metadata for Orchestrator routing.

## Report

Write `crates/haze-sync-server/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`
- `phase_id: SRV-P7B3-EXACT-SHA-FAN-IN`
- `chat_name: server — W1 SRV-P7B3 Exact-SHA Fan-In`

Use one honest status:

- `SELF_ACCEPT`
- `SELF_NEEDS_FIX`
- `SELF_BLOCKED_BY_CONTRACT`
- `SELF_BLOCKED_BY_SCOPE`
- `SELF_BLOCKED_BY_TOOLING`

`SELF_ACCEPT` requires exact owner product equality, no forbidden imports, fully green DB-capable CI and no implementation of SRV-P7B3 executor behavior.

After `SELF_ACCEPT`, mandatory clean-code review of the fan-in is required. Only after that review returns `CLEAN_ACCEPT` may the Orchestrator activate `server — W1 SRV-P7B3 Bounded Worktree Executor`.