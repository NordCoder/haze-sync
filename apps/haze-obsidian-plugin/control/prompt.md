# W1-OBS-P2 — Obsidian settings, secret handling, and lifecycle foundation

Component: obsidian-plugin
Component path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
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
- apps/haze-obsidian-plugin/docs/component-contract.md
- apps/haze-obsidian-plugin/docs/implementation-plan.md
- apps/haze-obsidian-plugin/docs/implementation-log.md
- apps/haze-obsidian-plugin/docs/dependency-map.md
- apps/haze-obsidian-plugin/docs/decisions.md
- apps/haze-obsidian-plugin/control/prompt.md
- relevant current code under apps/haze-obsidian-plugin/src/**

## Task

Implement phase OBS-P2 from the Obsidian plugin implementation plan: Settings, secret handling, and lifecycle foundation.

Goal:

```text
Implement safe plugin settings and lifecycle cleanup before adding network or sync behavior.
```

## Allowed component scope

```text
apps/haze-obsidian-plugin/src/**
apps/haze-obsidian-plugin/docs/**
apps/haze-obsidian-plugin/control/report.md
```

## Expected work

- Define PluginSettings for server URL, adapter identity, auth token, sync mode, and safety toggles.
- Add settings load/save through Obsidian APIs.
- Add settings tab with token redaction/masking.
- Validate server URL and mode values.
- Ensure onunload() removes listeners, timers, and status items introduced by this phase.
- Introduce safe notice/status reporting.
- Add unit-testable pure helpers where practical.

## Explicit non-goals

- No server sync calls.
- No vault scanner.
- No conflict UI.
- No Google Drive integration.
- No token rotation endpoint.
- No API contract redesign.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires storing tokens in unsafe global state, logging/displaying tokens, changing API auth scheme, or relying on mobile background behavior for correctness.

## Checks

Run applicable checks if available:

```text
npm install --no-audit --no-fund
npm run --workspace haze-obsidian-plugin typecheck
npm run --workspace haze-obsidian-plugin build
```

If Rust workspace changes unexpectedly occur, also run the Rust checks. If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
apps/haze-obsidian-plugin/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
