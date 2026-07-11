# Worktree doctor and repair boundary

## Scope

WT-P9 exposes Worktree-owned health facts and repair proposals. It does not host
HTTP routes, access Storage directly, execute destructive repair, or select a
concrete watcher. A later Server fan-in may adapt these types without moving
Worktree logic into Server.

## Fact flow

The doctor consumes an injected `WorktreeDoctorSnapshot` containing:

- reconciliation entries produced from a full scanner pass and persisted
  last-applied/observation state;
- the reconciliation count summary, including expired echo markers;
- the host-driven runtime status.

Watcher hints are never accepted as doctor facts. They may reduce scheduling
latency, but scan/reconciliation output remains the correctness source.

`WorktreeDoctorFactSource` is the persistence/hosting boundary. Server or another
accepted composition layer may load persisted state and runtime facts, but the
Worktree crate does not own database access.

## Safe output

`WorktreeDoctorReport` exposes:

- overall health: healthy, degraded, or unhealthy;
- count-only summary fields;
- stable issue categories;
- optional validated vault-relative paths.

It never contains configured-root absolute paths, raw filesystem errors, provider
payloads, database errors, or watcher event paths.

Covered categories include missing and dirty files, content-hash mismatches,
reserved-path violations, skipped symlinks/special/unsafe/filesystem entries,
stale or expired echo state, partial scans, and degraded runtime state.

## Repair planning

`WorktreeRepairPlanner` produces an explicit plan only. There is intentionally no
repair executor in WT-P9.

Non-destructive actions such as rescan, restoring a genuinely missing file, or
leaving a skipped entry unchanged are authorized in an unconfirmed plan.
Overwrite, move, trash, or delete risks require
`WorktreeRepairAuthorization::ConfirmedByHost`. The authorization records that a
higher-level accepted contract confirmed the proposal; it still does not execute
it.

A future host must revalidate current facts immediately before execution. Plans
must not be treated as durable authorization because filesystem and Core state can
change after diagnosis.

## Future Server fan-in

A dedicated cross-component phase may:

1. implement `WorktreeDoctorFactSource` using accepted Server/Storage boundaries;
2. map doctor enums and counts into safe admin/doctor DTOs;
3. host runtime lifecycle and expose status;
4. add an explicitly confirmed repair command path with fresh-fact revalidation;
5. add E2E coverage using a temporary real worktree.

That fan-in must not move scanner, reconciliation, doctor, repair-policy, or
runtime state-machine ownership into Server.
