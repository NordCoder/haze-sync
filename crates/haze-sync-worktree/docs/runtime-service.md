# Worktree Runtime Service

## Ownership

`WorktreeRuntimeService` is a host-driven library abstraction owned by the Worktree component.

It does not:

- start a Tokio task;
- create a process or listener;
- wire itself into Server startup;
- own Core/API clients;
- treat watcher events as filesystem facts.

A future Server/Worktree fan-in owns composition and repeatedly calls:

```text
start -> poll* -> request_cancel -> shutdown
```

Each `poll` runs at most one synchronous runtime cycle, so the service cannot overlap its own scans/imports/exports.

## Correctness and watcher hints

Watcher events are path-free scheduling hints. Their sequence values are diagnostic only.

The runtime remains correct when watcher events are:

- missing;
- duplicated;
- coalesced;
- reordered;
- unavailable because watcher startup or polling failed.

Startup and periodic cycles are independent of watcher delivery. Any mode that imports local state sets `full_scan_required` on every cycle. The host-provided cycle executor must run `WorktreeScanner` before deriving local import or delete facts.

Watcher failures degrade latency but do not stop periodic correctness cycles. Public watcher failure categories are normalized to the operation that observed them (`Start`, `Poll`, or `Shutdown`) rather than trusting a concrete watcher implementation to classify itself correctly.

## Modes

The runtime follows the project adapter-mode contract:

| Mode | Watch local files | Write local facts to Core | Apply Core exports |
| --- | --- | --- | --- |
| `Disabled` | no | no | no |
| `ReadOnly` | no | no | yes |
| `ImportOnly` | yes | yes | no |
| `ExportOnly` | no | no | yes |
| `Bidirectional` | yes | yes | yes |

`ReadOnly` means the adapter may read Core but may not write Core. It therefore has the same runtime direction capabilities as `ExportOnly`, while preserving the distinct configuration vocabulary.

## Bounded work

`WorktreeRuntimePolicy` bounds:

- watcher hints consumed per host poll;
- planned import actions per cycle;
- guarded delete candidates per cycle;
- applied export/materialization actions per cycle.

The runtime validates cycle summaries. It rejects:

- a missing required full scan;
- import work in a mode that forbids Core writes;
- export work in a mode that forbids Core reads/applies;
- work above the configured budget;
- submitted import counts greater than planned import counts.

A failed or rejected cycle is not immediately retried in a tight loop. The next automatic attempt is scheduled at the periodic cycle deadline.

## Cancellation and shutdown

Cancellation is cooperative and explicit. Once cancellation is requested, no new cycle starts.

Shutdown:

- calls watcher shutdown whenever watcher startup succeeded, including after the watcher stream closed or polling failed;
- does not call watcher shutdown when watcher startup itself failed;
- clears pending hints and deadlines;
- permanently transitions the service to `Shutdown`;
- does not permit implicit restart.

Watcher shutdown failure is represented as a safe `Shutdown` category and does not expose local paths or raw filesystem errors.

## Status safety

`WorktreeRuntimeStatus` exposes only:

- lifecycle and mode enums;
- watcher health category;
- pending/count metrics;
- last cycle cause;
- count-only cycle summaries or safe failure categories.

It never exposes the configured absolute worktree root, watched paths, raw watcher payloads, tokens, database URLs, or provider responses.
