# W1-SRV-API-P8-HTTP-FUNCTIONAL-REVIEW — Review Worktree admin HTTP wiring

Before starting, name this worker chat exactly:

`server — W1 API-P8 Worktree HTTP Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-API-P8-HTTP-FUNCTIONAL-REVIEW

Do not merge, change draft state, rewrite history, modify sibling branches, begin CLI-P6A, or perform unrelated cleanup.

## Candidate

- accepted Server baseline: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- accepted API-P8 SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- final Server code-bearing SHA: `be2b1c16fa6c4919d446b76b1f15dca5767b2482`;
- implementation report blob: `9b4244bb4d6f37edbca85d6ab809ad171999f4f1`;
- authoritative DB-capable Component CI run: `29321038276`, number `1938`, attempt `1`, success.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance.

## Review scope

Verify only substantive correctness, ownership, concurrency, authorization, HTTP semantics, secrecy, exact fan-in and protected scope.

### Exact API fan-in

1. Accepted API-P8 product blobs match exactly, including required `dto/public_contract_tests.rs` dependency.
2. No API control files were copied.
3. Accepted API public vocabulary and semantics were not edited.

### Runtime ownership and control handle

4. Startup retains unique shutdown/join ownership of the live Worktree host.
5. Cloneable HTTP control owns no strong host owner, task, join, shutdown sender, watcher, executor, DB pool, object store or root.
6. Weak-handle upgrade failure is handled safely and cannot bypass shutdown ownership.
7. `Arc::try_unwrap` or equivalent shutdown path cannot be defeated by route-held strong references.
8. Status performs one passive bounded snapshot only.
9. Sync performs one bounded authoritative submission only.
10. No mutex around the long-running host, no duplicate gate/lifecycle mirror, no task/poller/retry/completion watcher.
11. Dropping the accepted ticket does not cancel the queued owner request; no completion polling or waiting occurs.
12. Snapshot-to-submit races are resolved by the authoritative Worktree submission result.
13. Disabled production mode still exposes honest disabled/ready/inert/unavailable status.

### GET status route

14. Route is exactly `GET /v1/admin/worktree/status`.
15. Existing token verification and Admin authorization are required.
16. Mapping exactly preserves Server mode, lifecycle, readiness, reason, counters, cycle flag, watcher hints and manual availability.
17. Running+Busy remains Ready; Disabled is Ready/DisabledInert/Unavailable; Failed is NotReady/Failed/Failed.
18. Counters/hints are informational only.
19. Checked count conversion failure maps to fixed sanitized internal error.
20. Missing control returns fixed safe 503 and does not fabricate status.
21. GET performs no DB, filesystem, provider, probe or submission work.

### POST sync-once route

22. Route is exactly `POST /v1/admin/worktree/sync-once`.
23. Strict empty JSON request rejects malformed/unknown fields with fixed safe 400 InvalidRequest response.
24. Admin authorization is required; missing/invalid token and non-admin mappings remain 401/401/403.
25. Client cannot set budget, path, mode, force, ticket, generation or request id.
26. Server constructs the bounded DryRun request from validated host config.
27. Exact outcome mapping:
    - Accepted -> 202 / accepted;
    - Busy -> 409 / busy;
    - NotStarted, Cancelling, Shutdown, Unavailable -> 503;
    - Failed -> 500.
28. All submission outcomes use accepted API-P8 response vocabulary.
29. `accepted` means queued/submitted only and returns immediately.
30. No raw Worktree failure/contract violation is exposed.

### Tests, secrecy, scope and CI

31. Deterministic tests cover auth, strict body, every outcome, absent control, Disabled, Running+Busy, Failed, counters/hints, ticket-drop behavior and ownership.
32. Route output cannot expose paths, roots, fingerprints, DB URLs, provider/request payloads, raw errors, tokens/hashes, cursors, idempotency values, ticket or generation ids.
33. No Worktree runtime/gate/watcher/executor, Storage, Core, CLI or Deployment product file changed.
34. No migration or workflow changed.
35. Exact final SHA has green DB-capable CI and later commits before the review report are control-only.

Do not modify code unless there is a concrete functional, concurrency, authorization, secrecy or scope defect. No formatting-only corrections.

## Report

Write `crates/haze-sync-server/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: SRV-API-P8-HTTP-FUNCTIONAL-REVIEW`;
- `chat_name: server — W1 API-P8 Worktree HTTP Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker exists, use `CLEAN_ACCEPT` and authorize CLI-P6A control-slot resolution. Do not implement CLI work or claim merge readiness.
