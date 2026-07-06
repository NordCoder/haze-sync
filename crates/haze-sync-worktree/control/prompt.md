# W1-WT-P2C — Worktree clean-code review

Component: worktree
Component path: crates/haze-sync-worktree
Branch: component/worktree
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-worktree/control/state.md
- crates/haze-sync-worktree/control/report.md as the W1 WT-P2 implementation report before overwriting it
- crates/haze-sync-worktree/docs/component-contract.md
- crates/haze-sync-worktree/docs/implementation-plan.md
- changed files in component/worktree against main

## Task

Review W1 WT-P2 implementation. Verify root containment/path mapping, error redaction, reserved path policy, same-component Cargo scope expansion, no watcher/provider/runtime behavior, and report honesty.

## Allowed files

- crates/haze-sync-worktree/src/** only for small review fixes
- crates/haze-sync-worktree/docs/** only for small review fixes
- crates/haze-sync-worktree/Cargo.toml only if required for review fix
- crates/haze-sync-worktree/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-worktree/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
