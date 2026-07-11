# Dependency Map: worktree

## Component role in dependency graph

`haze-sync-worktree` is the built-in filesystem adapter for the VPS worktree.

Worktree owns materialized filesystem replica behavior, path-to-file operations, scan/watch planning, export/import mechanics for the local worktree, and filesystem-safe error handling.

Worktree does not own Core sync policy, API DTOs, Server route definitions, Storage schema, Google Drive behavior, Obsidian plugin behavior, Deployment automation, or CI workflow policy.

## Independent development model

`haze-sync-worktree` can be developed independently inside the `component/worktree` branch.

The dependency map records filesystem adapter contracts and fan-in points. It does not impose a serial implementation order on Core, Server, Storage, adapters, Deployment, or CI.

Allowed independent work includes:

- worktree path normalization and safety checks;
- scan/watch/materialization planning;
- filesystem apply/export helpers;
- local trash/staging mechanics where scoped;
- worktree status/doctor primitives;
- tests using temporary directories and safe fixtures.

If Worktree needs Core apply semantics, Server runtime mounting, Storage metadata, Deployment paths, or CI test support not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's responsibility.

## Upstream contracts consumed

Worktree may consume:

- Core input/result contracts for local file changes;
- Common shared IDs/types where accepted;
- Server runtime contracts when Worktree is mounted inside server;
- Deployment path/permission docs as operational configuration, not source-of-truth behavior.

Worktree must not consume:

- Google Drive provider APIs;
- Obsidian plugin internals;
- API DTOs as filesystem policy;
- Storage internals unless explicitly contracted through Core/Server integration;
- CI workflow logic.

## Downstream contracts exposed

Expected downstream consumers:

- Server, for built-in Worktree runtime composition;
- Core, through normalized local incoming changes and apply-result handling;
- Deployment, for host path and permission requirements;
- CLI/doctor, indirectly through Server/Core status surfaces;
- tests and future integration harnesses.

Downstream consumers must not treat Worktree files as hidden source-of-truth metadata.

## Forbidden dependency directions

Worktree must not:

- decide final conflict/delete outcomes outside Core;
- write hidden metadata that becomes authoritative over Core;
- call Google Drive or Obsidian APIs;
- expose raw local paths publicly without safe formatting;
- hard-delete files outside accepted retention/trash policy;
- own deployment service layout.

## Cross-component contracts

Important Worktree contracts:

- Worktree is a materialized view, not the source of truth;
- path handling must be safe and repository/vault-bound;
- local file changes are normalized before Core submission;
- Core apply results determine final local materialization behavior;
- scans provide correctness; watchers provide latency;
- filesystem errors are safe to report publicly.

## Integration/fan-in ownership

Fan-in is required when:

- Core input/apply-result contracts change;
- Server mounts Worktree runtime services;
- Deployment provisions worktree directories/permissions;
- Storage/Core need durable mapping or cursor behavior;
- CLI/doctor exposes Worktree health.

These are integration gates. They do not block independent Worktree work inside its component boundary.

## Dependency rules

- Worktree owns filesystem mechanics, not sync authority.
- Worktree submits normalized changes to Core rather than deciding overwrites.
- Worktree path safety must be local and strict.
- Worktree tests should not require GDrive, Obsidian, or production deployment state.
- Operational paths are configured by Deployment; Worktree validates and uses them safely.

## Contract-change notes

Current known contract questions:

1. Runtime mounting
   - Server owns composition; Worktree owns filesystem behavior.
   - Missing mounting contracts are Server/Worktree fan-in points.

2. Local delete/trash behavior
   - Worktree can implement mechanics.
   - Core owns delete/tombstone policy.

3. Scan/watch guarantees
   - Worktree can independently implement foundations.
   - Integration tests later verify end-to-end behavior with Core/Server.

No serial implementation dependency is implied by this map.
