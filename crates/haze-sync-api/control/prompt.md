# W1-API-P8-FUNCTIONAL-REVIEW — Review Worktree status and sync-once contract

Before starting, name this worker chat exactly:

`api — W1 API-P8 Worktree Status Review`

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: clean-code-reviewer
Phase: API-P8-FUNCTIONAL-REVIEW

Do not merge, change draft state, rewrite history, modify sibling branches, wire Server routes, or begin CLI-P6A.

## Candidate

- synchronized API baseline: `d0e8ef0705b7c0456f2cb1359428ff30b90961b4`;
- original API-P8 implementation SHA: `8eb6e0ce44612e1e2f111415026297df8fb1d82b`;
- final post-rustfmt code-bearing SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- implementation report commit: `8f430d8c05b5151b91039c280ea20cfe2b382f3e`;
- implementation report blob: `390fa0b676b57a1796b840eb0d63f9ebae2599b4`;
- fixer report commit: `59dea568b7bd5eed6dfb8b6edb1cdc6521cfc5b5`;
- fixer report blob: `08ed3e218e38a93782d3898369293b723eabac00`;
- authoritative Component CI run: `29315949762`, number `1925`, attempt `1`, success.

Accepted Server source contract:

- SRV-P7B5 SHA: `1d1fc8ca62c97db041cca09dd8316370285dfba1`;
- Server clean report commit: `23b49dff38c8b6997193b6224681feada3e09c1e`;
- Server clean report blob: `2416d7280883761bf90117a7e3dbfc41147756b9`.

Formatting, rustfmt, naming taste and stylistic matters are out of scope and must not block acceptance. The three final product-file changes after the implementation SHA were artifact-proven rustfmt output only.

## Review scope

Verify only substantive contract, compatibility, authorization, secrecy and scope correctness.

### Status DTO

1. Public mode vocabulary is exactly:
   `disabled`, `read_only`, `import_only`, `export_only`, `bidirectional`, `dry_run`.
2. Public lifecycle vocabulary is exactly:
   `disabled`, `starting`, `running`, `cancelling`, `shutdown`, `failed`.
3. Public readiness vocabulary is exactly `ready`, `not_ready`.
4. Readiness reasons are exactly:
   `disabled_inert`, `running`, `starting`, `cancelling`, `shutdown`, `failed`.
5. Manual availability is exactly:
   `available`, `busy`, `not_started`, `cancelling`, `shutdown`, `unavailable`, `failed`.
6. The response contains only the accepted safe fields: configured mode, lifecycle, readiness, readiness reason, completed/failed counters, cycle-in-progress, pending watcher hints and manual availability.
7. API accepts already-sanitized parts and does not independently derive or alter Server lifecycle/readiness/manual policy.
8. Disabled ready/inert, Running+Busy ready and Failed not-ready examples are represented accurately without claiming runtime validation by API.
9. Counters/hints are informational and cannot silently affect readiness.
10. Count conversion is deterministic and bounded; conversion errors expose no rejected values or internals.

### Sync-once submission contract

11. The contract requires an already verified Admin principal through a pure helper.
12. Missing principal and non-admin principal map to existing sanitized public auth/error vocabulary without adapter id, token or identity leakage.
13. Request shape is bodyless or strict-empty and rejects client-controlled budgets, paths, mode overrides, force flags, ticket/generation ids and unknown runtime fields.
14. Outcomes are exactly:
    `accepted`, `busy`, `not_started`, `cancelling`, `shutdown`, `unavailable`, `failed`.
15. `accepted` means submission/queue acceptance only, never cycle completion or success.
16. API does not infer mode availability or translate internal Worktree failures/contract violations into raw public details.
17. No ticket polling, completion wait, generation, retry, background task or runtime execution exists.

### Compatibility, passivity and secrecy

18. The dedicated versioned fixture exactly matches the Rust contract and its verifier is strict enough to detect field/value drift.
19. Public serialization and debug/error output cannot contain raw paths, roots, fingerprints, database URLs, provider payloads, errors, tokens, token hashes, cursors, idempotency values, request payloads, tickets or generation identifiers.
20. Recommended HTTP path constants/helpers do not register Axum routes or claim Server wiring exists.
21. No dependency on `haze-sync-server`, private Server enums, Worktree runtime execution, DB, object store, provider or filesystem behavior was added.
22. No Server/Worktree/Storage/Core/CLI/Deployment product file, migration or workflow was modified.
23. Exact-SHA CI is green and all commits after `56ae945...` before the review report are control-only.

Review product paths introduced or changed by API-P8, including:

- `crates/haze-sync-api/src/dto/worktree.rs`;
- `crates/haze-sync-api/src/dto/mod.rs`;
- `crates/haze-sync-api/src/routes/worktree.rs`;
- `crates/haze-sync-api/src/routes/mod.rs`;
- `crates/haze-sync-api/fixtures/worktree-contract-v1.json`;
- `crates/haze-sync-api/tests/worktree_compatibility_fixture.rs`;
- relevant API-P8 docs.

Do not modify code unless there is a concrete functional, authorization, compatibility, secrecy or protected-scope defect. No formatting-only corrections.

## Report

Write `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`;
- `phase_id: API-P8-FUNCTIONAL-REVIEW`;
- `chat_name: api — W1 API-P8 Worktree Status Review`;
- status `CLEAN_ACCEPT`, `CLEAN_NEEDS_FIX`, `CLEAN_BLOCKED_BY_CONTRACT`, `CLEAN_BLOCKED_BY_SCOPE`, or `CLEAN_BLOCKED_BY_TOOLING`.

If no substantive blocker exists, use `CLEAN_ACCEPT` and authorize:

1. Server HTTP fan-in/wiring of the accepted API-P8 contract;
2. CLI-P6A status and sync-once client work, subject to its own control-slot verification.

Do not implement either downstream phase yourself and do not claim merge readiness.
