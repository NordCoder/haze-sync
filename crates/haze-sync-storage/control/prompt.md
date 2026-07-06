# W1-STOR-P2 — Storage schema and row-model audit

Component: storage
Component path: crates/haze-sync-storage
Branch: component/storage
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
- crates/haze-sync-storage/docs/component-contract.md
- crates/haze-sync-storage/docs/implementation-plan.md
- crates/haze-sync-storage/docs/implementation-log.md
- crates/haze-sync-storage/docs/dependency-map.md
- crates/haze-sync-storage/docs/decisions.md
- crates/haze-sync-storage/control/prompt.md
- relevant current code under crates/haze-sync-storage/src/**
- migrations/** for read-only schema comparison unless explicitly editing migrations is required by this prompt

## Task

Implement phase STOR-P2 from the Storage implementation plan: Schema and row-model audit.

Goal:

```text
Audit schema metadata, migration filenames, and row models against the accepted system storage model before more repository and fan-in work depends on them.
```

## Allowed component scope

```text
crates/haze-sync-storage/src/schema/**
crates/haze-sync-storage/src/models/**
crates/haze-sync-storage/docs/**
crates/haze-sync-storage/control/report.md
migrations/** only if explicitly required and reported as a schema change
```

## Expected work

- Verify expected tables are represented in schema table-name metadata.
- Verify initial migration filename ordering matches actual migration files.
- Audit row models for field names, optionality, timestamps, and JSON metadata fields.
- Clarify which fields may contain sensitive values and must not be returned publicly.
- Add row serialization/model tests where useful for fixtures.
- Document schema drift or missing row models.

## Explicit non-goals

- No migration runner.
- No Core policy.
- No API DTO mapping.
- No Server route wiring.
- No SQL query fan-in unless needed only for model/schema audit.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing migration order, adding/removing tables, changing persisted field meaning, or exposing row models directly as public API output.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-storage
cargo test -p haze-sync-storage
cargo clippy -p haze-sync-storage --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-storage/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
