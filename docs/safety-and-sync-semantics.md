# Safety and sync semantics

## Global invariants

Haze Sync V1 preserves data before optimizing convenience.

The global invariants are:

```text
Core is the only conflict arbiter.
Adapters never silently overwrite.
Every write has base_revision_id or explicit base_revision_id = null.
Unknown base revision never overwrites different existing content.
Delete is tombstone + trash + retention, never immediate hard delete.
Conflict does not block sync; preserve both by default.
Bootstrap starts from exactly one trusted source.
Watchers are for latency; scans are for correctness.
```

These invariants apply to all components. Component contracts may refine them locally, but must not weaken them.

## File identity

V1 uses path-based file identity inside one vault.

Implications:

- paths must be normalized before reaching Core decisions;
- absolute paths are not vault paths;
- `..` traversal and platform-specific escape paths are invalid;
- rename/move is treated as delete + create in V1;
- file content is synchronized atomically as a whole.

## Content identity

File content is identified by SHA-256.

Uploads must include the expected content hash. Downloads should return the revision and content hash. Hash mismatch is a safety error, not a recoverable normal outcome.

The content-addressed object store may deduplicate identical blobs, but deduplication must not collapse distinct file revisions or erase operation history.

## Base revision semantics

Every overwrite-capable write must include a base revision.

Allowed base forms:

```text
base_revision_id = current revision
base_revision_id = old/stale revision
base_revision_id = null, explicitly meaning unknown/no known base
```

Expected behavior:

```text
file missing + base=null
  -> accepted as initial/imported file

base=current + content differs
  -> accepted as new revision

current content hash == incoming content hash
  -> ignored as same_content

base=old/stale + content differs
  -> conflict_saved

base=null + file exists + content differs
  -> conflict_saved

unknown/untrusted base + different content
  -> conflict_saved or rejected, never silent overwrite
```

Adapters must not infer that missing local state means it is safe to overwrite server state.

## Upload outcomes

A write-like request may produce these conceptual outcomes:

```text
accepted
  Core created a new current revision and operation log entry.

ignored
  The request was safe but did not create a new revision, commonly same_content.

conflict_saved
  Core preserved current state and stored incoming content as a conflict/backup copy.

rejected
  The request was invalid, unsafe, unauthorized, too large, or out of policy.
```

Adapters must handle all outcomes explicitly.

## Conflict policy

A conflict occurs when incoming content differs from current content and the incoming base is not the current revision.

Default V1 policy:

```text
preserve_both
```

Behavior:

```text
1. Current main file remains current.
2. Incoming content is saved as a conflict copy.
3. A conflict record is created.
4. Operation log records the conflict.
5. Sync continues.
```

V1 may also use:

```text
current_wins_with_incoming_backup
```

This is semantically similar to preserve-both, but communicates that current state remains authoritative while incoming content is retained as backup/conflict material.

Excluded V1 policies:

```text
latest_wins
earliest_wins
incoming_wins
adapter_priority_wins
three_way_merge
semantic_merge
```

These policies are excluded because they increase silent data-loss risk and implementation complexity.

## Conflict materialization

Conflict copies may be materialized inside the vault under `_haze_conflicts/` so Obsidian, Google Drive, and the worktree can surface them without a separate Web UI.

Expected pattern:

```text
_haze_conflicts/open/<original-dir>/<stem>.conflict.<adapter-id>.<timestamp><ext>
```

Conflicts inside `_haze_conflicts/**` must not recursively create unbounded nested conflict paths.

## Conflict resolution actions

V1 public action vocabulary:

```text
accept_current
accept_conflict
keep_both
mark_resolved
```

Expected meanings:

```text
accept_current
  Keep current main file unchanged and resolve the conflict record.

accept_conflict
  Backup current main content, create a new main-file revision from conflict content,
  and resolve the conflict record.

keep_both
  Keep main file and conflict copy as separate files, then resolve the conflict record.

mark_resolved
  Only close metadata after an external/manual resolution.
```

Partial implementations must report their limitations honestly. For example, metadata-only resolution must not claim to replace file content.

## Delete policy

V1 never hard-deletes immediately.

Delete flow:

```text
external delete observed
  -> delete candidate
  -> guard checks
  -> tombstone
  -> adapter trash/materialized removal
  -> retention window
  -> later cleanup only when explicitly implemented and safe
```

A stale delete must not erase a newer edit. If a delete races with an edit, changed content should win by default and the delete should become a conflict/candidate or safe rejection.

## Delete guard

Adapters and Core-side delete handling must guard against accidental mass deletion.

Default conceptual guardrails:

```text
max_deletes_per_run
max_delete_ratio_per_run
manual unlock for mass delete
```

When a provider scan or local scan observes a dangerous delete burst, the adapter should stop propagation and report unsafe mass delete. It must not create tombstones merely because an external provider suddenly reports many missing files.

## Tombstones and retention

Tombstones represent logical deletion. Historical blobs and revisions remain available for retention, doctor checks, restore support, and auditability.

Physical cleanup is a separate future/background operation and must not be silently bundled into normal delete handling.

## Watchers and scans

Watchers are latency optimizations. Full scans are correctness mechanisms.

Consequences:

- every adapter that observes external state must eventually have a full scan/reconciliation path;
- watcher/webhook events may only enqueue or accelerate work;
- cursors should advance only after durable successful processing;
- invalid provider cursors must fall back to scan/reconciliation rather than dropping state.

## Echo guards

Adapters that export changes will often observe their own writes later.

Echo guard responsibilities:

- recognize writes performed by the adapter;
- avoid re-importing identical own writes as new edits;
- avoid suppressing real remote changes;
- preserve cursor and mapping consistency.

Echo guards are adapter-local correctness mechanisms. They do not replace Core base revision checks.

## Adapter modes

V1 uses adapter modes to support staged rollout:

```text
disabled
read_only
dry_run
import_only
export_only
bidirectional
```

Mode enforcement must be visible to operators and respected by adapters.

Safe interpretation:

```text
disabled
  no runtime sync work

read_only
  observe/report only, no Core writes and no provider writes

dry_run
  compute intended actions and reports without mutation

import_only
  external replica -> Core only

export_only
  Core -> external replica only

bidirectional
  both directions, with Core safety semantics and adapter guards
```

## Idempotency

Every write/delete request must use an idempotency key.

Expected behavior:

```text
same key + same request
  -> same stored outcome

same key + different request
  -> conflict/idempotency mismatch error
```

Idempotency records must not expose raw keys in public reports or logs.

## Public error safety

Public outputs must not expose:

- bearer tokens;
- OAuth tokens;
- token hashes;
- raw provider payloads;
- raw SQL/database errors;
- stack traces;
- local absolute paths outside configured public-safe roots;
- idempotency key values;
- secret configuration.

Errors should be mapped to safe codes and short messages.

## Security model

V1 is single-user and adapter-token based.

Adapter tokens should be scoped by role/mode. A token for one adapter should not imply permission to perform unrelated adapter operations or admin actions.

Real secrets and production `.env` files must not be committed. CI must not require production provider credentials.
