# W1-SRV-STOR-TEST-SUPPORT-FAN-IN — Restore Storage production feature isolation

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

Storage STOR-P9C completed its in-scope review with green CI, but found a Server-owned dependency-contract violation.

- Storage review head: aa59064d641f4850f7c70fa615e638b52613dd95
- Storage CI run: 29124956486
- Storage verdict: CLEAN_BLOCKED_BY_CONTRACT
- Server manifest evidence: crates/haze-sync-server/Cargo.toml currently enables `haze-sync-storage` feature `test-support` under normal dependencies

This task is an explicit cross-component fan-in correction within Server ownership. It does not unblock or begin SRV-P7 runtime composition.

## Read

Read the project process files, current Server control files, crates/haze-sync-server/Cargo.toml, Server tests and any use of Storage test-support, Storage STOR-P9C report and test-support documentation, Cargo feature/resolver configuration, and PR diff.

## Task

Restore the accepted Storage production-isolation boundary while preserving Server tests.

- keep Server's normal `haze-sync-storage` dependency free of `test-support`;
- enable `test-support` only for Server dev/test targets or a dedicated test harness;
- ensure normal Server production compilation does not export or require Storage test-support APIs;
- preserve existing Server test behavior and optional/required database-test semantics;
- make only the minimum Cargo/test-boundary changes needed;
- run normal Component CI and report the final code-bearing SHA and run.

## Allowed files

- crates/haze-sync-server/Cargo.toml
- crates/haze-sync-server/src/** only if a test-only cfg/import adjustment is directly required
- crates/haze-sync-server/tests/** only if directly required
- crates/haze-sync-server/docs/** only if dependency placement needs documentation
- crates/haze-sync-server/control/report.md

## Boundaries

No SRV-P7 runtime composition, Worktree hosting, new endpoints, Storage source changes, schema/migration changes, provider behavior, workflow changes, unrelated dependency upgrades, sibling changes, test deletion, or assertion weakening.

Product/Cargo/test changes must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-server/control/report.md using REPORT_TYPE IMPLEMENTATION and phase_id SRV-STOR-TEST-SUPPORT-FAN-IN.
