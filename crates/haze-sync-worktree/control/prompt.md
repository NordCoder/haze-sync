# W1-WT-P2 — Worktree config and path-mapping foundation

Component: worktree
Component path: crates/haze-sync-worktree
Branch: component/worktree
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
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- crates/haze-sync-worktree/docs/implementation-log.md
- crates/haze-sync-worktree/docs/dependency-map.md
- crates/haze-sync-worktree/docs/decisions.md
- crates/haze-sync-worktree/control/prompt.md
- relevant current code under crates/haze-sync-worktree/src/**

## Task

Implement phase WT-P2 from the Worktree implementation plan: Worktree config and path-mapping foundation.

Goal:

```text
Implement safe configuration and path mapping between `VaultPath` and local filesystem paths under one configured worktree root.
```

## Allowed component scope

```text
crates/haze-sync-worktree/src/**
crates/haze-sync-worktree/docs/**
crates/haze-sync-worktree/control/report.md
```

## Expected work

- Introduce `WorktreeConfig` or equivalent.
- Represent configured worktree root without exposing it in public errors.
- Map `VaultPath` to local file paths under root.
- Map local paths back to `VaultPath` only when they are inside root.
- Reject traversal, absolute-path escape, backslash escape, and unsafe containment cases within this phase scope.
- Define reserved runtime directories such as temp/trash/metadata/echo areas if needed.
- Add tests for path normalization, root containment, and reserved paths.

## Explicit non-goals

- No scanner loop.
- No Server config loading.
- No API calls.
- No Core policy.
- No filesystem watcher.
- No provider/GDrive/Obsidian behavior.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires allowing paths outside root, following symlinks by default, changing Common `VaultPath` semantics, exposing local absolute paths in public errors, or reserving paths that conflict with Core conflict materialization.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-worktree
cargo test -p haze-sync-worktree
cargo clippy -p haze-sync-worktree --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-worktree/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
