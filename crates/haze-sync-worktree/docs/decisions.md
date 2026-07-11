# Decisions: worktree

## 2026-07-05 — Worktree is a materialized replica, not source of truth

Decision:

`haze-sync-worktree` treats Core metadata/object-store state as authoritative. Local filesystem state is observed as facts and submitted through Core/API semantics; it is not an overwrite authority.

Rationale:

The worktree exists so local tools and agents can interact with files, but Core must remain the only conflict/delete/revision arbiter. Treating local files as authoritative would reintroduce silent overwrite and unsafe delete risks.

Alternatives:

- Let Worktree overwrite Core based on local mtimes.
- Treat Worktree as the canonical source and export to other replicas.
- Resolve conflicts locally in the filesystem adapter.

Consequences:

- Worktree imports must carry base/null-base semantics.
- Worktree materialization must protect dirty local files.
- Worktree scanner/importer submits facts; Core/API decides outcomes.

Affected contracts:

- component contract;
- dependency map;
- import planner phases;
- materializer phases;
- Server-hosted runtime fan-in.

## 2026-07-05 — Scans are correctness; watchers are latency

Decision:

Worktree may use filesystem watcher events as latency hints, but full scans remain the correctness mechanism.

Rationale:

Filesystem watchers can drop, coalesce, duplicate, or reorder events. Correct sync must not depend on watcher reliability.

Alternatives:

- Rely only on watcher events.
- Poll continuously without watcher support.
- Treat missed watcher events as unrecoverable errors.

Consequences:

- Scanner must be implemented before or alongside watcher runtime.
- Watcher events should schedule/debounce scans rather than directly mutate Core.
- Tests should prove scan behavior independently from watcher behavior.

Affected contracts:

- scanner;
- watcher/runtime service;
- doctor/repair planning;
- Server-hosted lifecycle.

## 2026-07-05 — Worktree path mapping is root-contained and conservative

Decision:

Worktree path mapping must keep all filesystem operations under one configured root and reject traversal, absolute-path escape, symlink/special-file behavior, and reserved runtime paths unless a future contract explicitly accepts a narrower behavior.

Rationale:

The filesystem is adversarial enough for sync purposes. Path escape or symlink-following bugs could read/write/delete outside the vault worktree.

Alternatives:

- Trust local filesystem paths after string prefix checks.
- Follow symlinks by default.
- Store adapter runtime metadata in user-visible arbitrary paths.

Consequences:

- Path mapping and scanner phases need strong tests.
- Reserved runtime paths must be documented and coordinated with Core conflict paths.
- Public errors should prefer vault-relative context over local absolute paths.

Affected contracts:

- path mapping;
- scanner;
- materializer;
- local trash/runtime metadata;
- Common `VaultPath` semantics.

## 2026-07-05 — Materialization must be atomic and dirty-file safe

Decision:

Worktree materialization should write Core revisions through safe temp/rename discipline and must not silently overwrite locally dirty files.

Rationale:

Worktree is a replica, but users/agents may edit files locally. Materialization that overwrites dirty files would destroy local work before Core can preserve both sides.

Alternatives:

- Directly overwrite target files.
- Assume local files are never edited during materialization.
- Let Server decide filesystem overwrite behavior.

Consequences:

- Worktree needs dirty-state and echo-guard modeling.
- Conflict/import planning may be triggered when materialization finds local divergence.
- Atomic writer behavior needs platform-aware tests.

Affected contracts:

- materializer;
- atomic writer;
- echo guard;
- import planner;
- Core conflict policy.

## 2026-07-05 — Echo suppression is Worktree-owned

Decision:

Worktree owns echo suppression for files it writes during materialization. Adapter-written files must not be immediately re-imported as new local edits.

Rationale:

Without echo suppression, the adapter can create feedback loops: Core change materializes to disk, scanner sees it, then re-submits it as a local edit.

Alternatives:

- Let Server suppress worktree echoes.
- Rely on same-content Core ignores only.
- Disable scanning after every materialization for a fixed window.

Consequences:

- Worktree state must track last applied revision/hash/mtime or equivalent.
- Echo guard must be narrow enough not to hide real user edits.
- Doctor summaries should expose echo/drift state safely.

Affected contracts:

- worktree state;
- scanner;
- materializer;
- Server/admin status summaries;
- Storage `worktree_state` persistence if used.

## 2026-07-05 — Server may host lifecycle; Worktree owns behavior

Decision:

In V1, Server may host the Worktree runtime lifecycle, but scanner/importer/materializer/echo/trash/repair behavior remains in the Worktree component.

Rationale:

The architecture allows a built-in worktree adapter inside the server process for deployment simplicity. That should not collapse component ownership or move filesystem semantics into Server routes.

Alternatives:

- Put all Worktree runtime logic directly in Server.
- Run Worktree only as a separate process in V1.
- Let Worktree create its own HTTP server/runtime independent of Server composition.

Consequences:

- A dedicated Server/Worktree fan-in phase is required.
- Worktree should expose hostable services/traits rather than Server-specific internals.
- Server status/readiness can include Worktree summaries after integration.

Affected contracts:

- Worktree runtime service;
- Server startup/composition;
- deployment topology;
- operational runbook.

## 2026-07-05 — Local delete behavior is trash/retention-oriented

Decision:

Worktree delete behavior must align with Core tombstones and system retention policy. Immediate irreversible hard delete is out of scope unless explicitly accepted by a future contract.

Rationale:

Deletes are the highest-risk filesystem operation. Local deletion must remain recoverable and coordinated with Core/API delete guard behavior.

Alternatives:

- Delete files immediately when Core tombstones them.
- Propagate local deletes without Core delete guard.
- Let Worktree cleanup retention independently from Core.

Consequences:

- Local trash/backup design is required before destructive behavior.
- Delete candidates from scans are submitted as facts.
- Hard-delete cleanup requires a separate operational contract.

Affected contracts:

- delete/trash phase;
- Core tombstones/delete guard;
- Storage worktree state;
- Server/CLI repair and cleanup planning.
