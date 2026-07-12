# W1-API-P8-BLOCKED-BY-SRV-P7B4 — Await accepted hosted Worktree status vocabulary

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: none
Phase: API-P8-BLOCKED-BY-SRV-P7B4

This is a hold notice, not an executable worker prompt.

The existing API component-local plan through API-P7C is `CLEAN_ACCEPT` at code-bearing SHA:

`3109c0fd9b456ca5fd8db099cd83843dae44cef9`

Authoritative Component CI run `29093081652`, run number `1217`, concluded successfully.

The next planned API phase is `API-P8 — Passive Worktree Runtime Status Contract`.

API-P8 must not begin until:

1. Server exact-SHA Worktree/Storage fan-in is clean-accepted;
2. `SRV-P7B3` real bounded executor is clean-accepted;
3. `SRV-P7B4` hosted Worktree runtime is clean-accepted;
4. the internal hosted status/lifecycle/readiness vocabulary is stable enough for API to represent passively.

API must not invent runtime lifecycle, readiness, manual-cycle or failure semantics ahead of Server. When unblocked, API-P8 will own only safe passive DTOs/parsers/errors and compatibility tests; it will not implement runtime, database, filesystem or provider behavior.

Before future API-P8 work, synchronize `component/api` with current `main` through an explicit Orchestrator sync because the long-lived API branch is not the current integration line.

Do not launch an API worker from this notice.