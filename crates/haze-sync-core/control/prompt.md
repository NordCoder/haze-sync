# W1-CORE-P2 — Core public module audit

Component: core
Component path: crates/haze-sync-core
Branch: component/core
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
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/docs/decisions.md
- crates/haze-sync-core/control/prompt.md
- relevant current code under crates/haze-sync-core/src/**

## Task

Implement phase CORE-P2 from the Core implementation plan: Public module audit and behavior/test alignment.

Goal:

```text
Audit every public Core module against the component contract, then add small rustdoc/test hardening where current behavior is underdocumented or undertested.
```

## Allowed component scope

```text
crates/haze-sync-core/src/**
crates/haze-sync-core/tests/** if present or added inside the component
crates/haze-sync-core/docs/**
crates/haze-sync-core/control/report.md
```

## Expected work

- Verify `src/lib.rs` exports only intended Core modules.
- Audit public types/functions for safe serialization and public output.
- Add missing tests for current behavior without changing semantics.
- Improve rustdoc where a type may be mistaken for persistence/runtime ownership.
- Mark any discovered public API mismatch as a contract-change request.

## Explicit non-goals

- No new sync behavior.
- No downstream wiring.
- No sibling crate edits.
- No SQLx, Axum, provider, filesystem, or runtime dependencies.
- No route/storage/server/adapters implementation.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing Core public semantics, moving transaction/runtime ownership into Core, or editing another component.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-core
cargo test -p haze-sync-core
cargo clippy -p haze-sync-core --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-core/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
