# W1-CLI-P2 — CLI parser and output contract hardening

Component: cli
Component path: crates/haze-sync-cli
Branch: component/cli
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
- crates/haze-sync-cli/docs/component-contract.md
- crates/haze-sync-cli/docs/implementation-plan.md
- crates/haze-sync-cli/docs/implementation-log.md
- crates/haze-sync-cli/docs/dependency-map.md
- crates/haze-sync-cli/docs/decisions.md
- crates/haze-sync-cli/control/prompt.md
- relevant current code under crates/haze-sync-cli/src/**

## Task

Implement phase CLI-P2 from the CLI implementation plan: Parser, command model, and output contract hardening.

Goal:

```text
Harden the current parser and output model before adding live commands or richer formats.
```

## Allowed component scope

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
crates/haze-sync-cli/control/report.md
```

## Expected work

- Audit top-level parser and doctor parser for consistent command model style.
- Add explicit exit-code mapping helpers if useful.
- Add tests for stdout/stderr expectations where practical.
- Ensure parse errors do not echo sensitive-looking arguments unsafely.
- Decide whether to keep the manual parser or request a contract change for a parser dependency.
- Document command categories: read-only, network-backed, mutation, destructive.

## Explicit non-goals

- No live server calls.
- No config loading.
- No provider calls.
- No repair/mutation behavior.
- No Server/API/Core/Storage edits.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires command syntax implying mutation, exposing raw secret-like argument values, changing documented command names, or adding a parser dependency without review.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-cli
cargo test -p haze-sync-cli
cargo clippy -p haze-sync-cli --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-cli/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
