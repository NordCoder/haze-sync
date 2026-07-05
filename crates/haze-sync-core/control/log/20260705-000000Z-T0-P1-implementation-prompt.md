# T0-P1 — Core component contract audit

Component: core
Component path: crates/haze-sync-core
Branch: component/core
Base branch: main
Verified base SHA: aabf74486d4d06a89136007bd17713f3c35478de
Target branch: main

## Role

You are an Implementation Worker. Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/docs/decisions.md
- crates/haze-sync-core/control/README.md
- crates/haze-sync-core/control/state.md
- current crates/haze-sync-core source code
- crates/haze-sync-common source only as dependency context

## Goal

Replace generic Core scaffold docs with useful current-state component documentation. This is a process test, not a feature task.

## Allowed files

- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/docs/decisions.md
- crates/haze-sync-core/control/state.md
- crates/haze-sync-core/control/report.md

Do not change Core source unless it is a tiny comment/doc typo fix. Do not change files outside crates/haze-sync-core.

## Required output

Update Core docs to describe current post-W3 responsibilities, public modules, owned behavior, non-goals, dependencies, safety/secrecy rules, test obligations, risks, and deferred work.

The contract must state that Core does not own Axum routing, SQLx repositories, provider/adapter calls, or broad runtime side effects.

Add one T0-P1 entry to implementation-log.md. Add one decision explaining this process test. Update control/state.md to PROMPT_READY or REPORT_READY as appropriate.

Write the final report to crates/haze-sync-core/control/report.md using report-template.md.

## Checks

Run or honestly mark not run:

- cargo fmt --check
- cargo check -p haze-sync-core
- cargo test -p haze-sync-core

Expected final status: SELF_ACCEPT or SELF_ACCEPT_PENDING_CI.
