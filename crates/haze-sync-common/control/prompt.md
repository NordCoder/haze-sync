# W1-CMM-P2 — Common public primitive audit

Component: common
Component path: crates/haze-sync-common
Branch: component/common
Base branch: main
Current main baseline SHA: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
Target branch: main

## Role

You are an Implementation Worker for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read before editing

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- docs-process/docs/component-docs-guide.md
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- crates/haze-sync-common/docs/implementation-log.md
- crates/haze-sync-common/docs/dependency-map.md
- crates/haze-sync-common/docs/decisions.md
- crates/haze-sync-common/control/prompt.md
- relevant current code under crates/haze-sync-common/src/**

## Task

Implement phase CMM-P2 from the common implementation plan: Public primitive audit and doc/test alignment.

Goal:

```text
Audit current common primitives against the documented contract and close small gaps in tests or rustdoc without changing intended behavior.
```

## Allowed component scope

```text
crates/haze-sync-common/src/**
crates/haze-sync-common/docs/**
crates/haze-sync-common/control/report.md
```

You may make broader edits inside the common component only when required for a clean implementation. Do not edit sibling components.

## Expected work

- Verify all public re-exports match the common contract.
- Clarify rustdoc where validation, representation, and policy could be confused.
- Add missing unit tests for current primitive edge cases.
- Verify serde wire values are stable.
- Verify formatting/debug output does not leak sensitive values.
- Document any discovered contract mismatch in common docs rather than silently changing downstream contracts.

## Explicit non-goals

- No new runtime behavior.
- No sibling crate edits.
- No new dependency on Core/API/Storage/Server.
- No broad naming churn unless required for contract correctness.
- No provider, server, storage, adapter, or CLI behavior.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if the work requires changing shared public wire formats, adding new shared ID/hash semantics, or changing path/security behavior beyond current component scope.

## Checks

Run applicable checks if your environment supports them. At minimum, prefer:

```text
cargo fmt --check
cargo check -p haze-sync-common
cargo test -p haze-sync-common
cargo clippy -p haze-sync-common --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, state that honestly in the report. Do not claim checks passed unless actually run or observed through CI.

## Report

Write the final report to:

```text
crates/haze-sync-common/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
