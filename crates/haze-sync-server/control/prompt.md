# W1-SRV-STOR-TEST-SUPPORT-FAN-IN-C — Storage feature-isolation clean-code review

Before starting, name this worker chat exactly:

`server — W1 SRV-STOR-TEST-SUPPORT-FAN-IN-C Clean-Code Review`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

The Server-owned Storage production feature-isolation correction is implemented and has green CI.

- code_bearing_sha: dc53f8dbe08da56d129fc3898cec262149c69f38
- workflow: Component CI
- workflow_run_id: 29127012776
- run_number: 1616
- conclusion: success

Current manifest evidence places `haze-sync-storage` without `test-support` in normal dependencies and enables `test-support` only in Server dev-dependencies.

This review is limited to the dependency correction. It does not begin SRV-P7 runtime composition.

## Read

Read the project process files, current Server control files, Server Cargo manifest, workspace Cargo resolver configuration, all Server source/tests that use Storage test-support, Storage STOR-P9C report and documentation, the implementation diff, and PR diff. Do not read diagnostics artifacts unless a later fixer prompt explicitly requires them.

## Task

Review the Storage test-support feature-isolation correction.

Focus on:

- normal Server production dependency graph not enabling Storage `test-support`;
- Cargo resolver-v2 behavior and duplicate normal/dev dependency declarations;
- Server unit/integration tests retaining required test-support access;
- absence of production imports or exports of test-only APIs;
- no accidental dependency upgrade or feature expansion;
- preservation of optional versus mandatory database-test semantics;
- minimal Cargo/test-boundary scope and clear documentation if needed.

## Allowed files

- crates/haze-sync-server/Cargo.toml
- crates/haze-sync-server/src/** only if a test-only cfg/import correction is directly required
- crates/haze-sync-server/tests/** only if directly required
- crates/haze-sync-server/docs/** only if dependency placement needs clarification
- crates/haze-sync-server/control/report.md

## Boundaries

No SRV-P7 runtime composition, Worktree hosting, new endpoints, Storage source changes, schema/migration changes, provider behavior, workflow changes, unrelated dependency upgrades, sibling changes, test deletion, or assertion weakening.

Source/Cargo/test/docs review commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-server/control/report.md using REPORT_TYPE CLEAN_CODE_REVIEW and phase_id SRV-STOR-TEST-SUPPORT-FAN-IN-C.
