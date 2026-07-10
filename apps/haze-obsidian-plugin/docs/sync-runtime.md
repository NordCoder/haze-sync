# Obsidian sync runtime

## Explicit runner

`Sync now` is the primary sync entry point. The runner serializes scan, push, pull,
and conflict refresh work so two sync runs cannot overlap.

The existing scan-only and pull-only commands use the same runner lock. Conflict
resolution remains a separate explicit user action and is disabled while a sync run
is active.

## Mode behavior

- `disabled`: no full or automatic sync runs.
- `pull_only`: full sync scans local state, pulls verified remote changes, and refreshes conflicts.
- `push_only`: full sync scans, pushes eligible pending changes, and refreshes conflicts.
- `bidirectional`: full sync scans, pushes, pulls, then refreshes conflicts.
- `dry_run`: full sync scans and reads the open-conflict list, but performs no server or vault-content mutations.

Remote materialization still verifies revision/hash metadata and preserves dirty local
files. Push requests retain persisted idempotency keys. Pending entries are removed
only after a confirmed server outcome, and a newer event for the same path is not
cleared by an older in-flight request.

Server delete/tombstone requests are not automated while `Confirm delete actions` is
enabled. Those pending delete entries remain queued. Disabling that safety setting is
an explicit choice to allow the runner to submit delete requests through Haze Sync
Server; it does not perform a local hard delete.

## Automatic triggers

Interval and vault-event triggers are optional and off by default. Event notifications
are only hints: an event-triggered run still performs a full vault scan before planning
mutations.

Automatic triggers and retries are best-effort only while Obsidian and the plugin stay
active. They are not a mobile background-service guarantee. Mobile operating systems
may suspend Obsidian, timers, networking, and event delivery.

## Offline and backoff behavior

The plugin persists sanitized sync runtime state, including the last trigger, last
success/failure timestamps, failure category, consecutive failure count, and next retry
time.

Offline, rate-limited, and server-unavailable failures use bounded exponential backoff.
When any automatic trigger is enabled, the runner owns a retry timer and cancels it on
reconfiguration or unload. Manual `Sync now` may retry immediately even during
backoff. Non-retryable configuration, authorization, invalid-response, or internal
failures remain visible and do not start an automatic retry loop.

## Lifecycle and secrecy

The runner owns interval, debounce, follow-up, and retry timers. Plugin unload clears
all timers, aborts the active HTTP requests through `AbortSignal`, suppresses further
work, and marks the runtime stopped in local state.

Status and notices contain only sanitized summaries. They do not expose auth tokens,
idempotency keys, raw server bodies, stack traces, provider payloads, database URLs,
or local absolute filesystem paths.
