# W1-FIX-CLI-P6A-TEST-REGRESSION

Before starting, name this worker chat exactly:

`cli — W1 CLI-P6A Test Restoration Fix`

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: fixer-worker
Phase: FIX-CLI-P6A-TEST-REGRESSION

Do not merge, change draft state, rewrite history, modify sibling branches, begin Deployment work, or perform unrelated cleanup.

Candidate:
- synchronized baseline `8a3012a20440066422e7ad6c4e52d1a859b1bd51`;
- current code-bearing SHA `70c3567f587a249a180eb8b9abb155065d197e5c`;
- review report blob `93327542ab208bd57521b6372e29ac14f253a616`;
- current green CI run `29342522606`, number `1948`.

Blocking defects:
1. Pre-existing CLI P1-P5 regression tests were deleted from `commands.rs` and `main.rs`.
2. Explicit successful Worktree status rendering assertions for Disabled and Failed lifecycle responses are missing.

Required focused fix:
- restore the deleted tests from exact synchronized baseline `8a3012a...` in `crates/haze-sync-cli/src/commands.rs` and `crates/haze-sync-cli/src/main.rs`;
- adapt restored exhaustive matches only as narrowly required for the new Worktree command variant;
- preserve all legacy expected behavior for global status, adapters, doctor, help, offline modes, placeholder annotation and safe parse errors;
- add explicit tests proving valid HTTP 200 Disabled and Failed Worktree status bodies render with success exit classification and accepted fields;
- keep the existing Running+Busy readiness `ready` assertion;
- preserve all current Worktree command/outcome/secrecy tests;
- do not change product behavior unless a restored test demonstrates a real defect;
- keep all accepted API-P8 blobs byte-identical.

Expected product scope is tests in:
- `crates/haze-sync-cli/src/commands.rs`;
- `crates/haze-sync-cli/src/main.rs`;
- `crates/haze-sync-cli/src/worktree_api.rs` only for focused Disabled/Failed tests.

Forbidden:
- API semantic edits;
- Server/Worktree/Storage/Core/GDrive/Deployment changes;
- migrations/workflows;
- broad refactors;
- lint/test suppression;
- polling, retry, waits, tasks or fake-success transport.

Create a real code-bearing commit without CI skip and obtain authoritative Component CI success on the exact final SHA.

Write `crates/haze-sync-cli/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-CLI-P6A-TEST-REGRESSION`;
- `chat_name: cli — W1 CLI-P6A Test Restoration Fix`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record restored test groups, any narrow adaptations, Disabled/Failed assertions, changed paths, accepted API blob verification, final SHA and exact CI. Do not claim CLEAN_ACCEPT; a final focused review follows.
