# W1-OBS-P2C — Obsidian plugin clean-code review

Component: obsidian-plugin
Component path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- apps/haze-obsidian-plugin/control/state.md
- apps/haze-obsidian-plugin/control/report.md as the W1 OBS-P2 implementation report before overwriting it
- apps/haze-obsidian-plugin/docs/component-contract.md
- apps/haze-obsidian-plugin/docs/implementation-plan.md
- changed files in component/obsidian-plugin against main

## Task

Review W1 OBS-P2 implementation. Verify settings/lifecycle code quality, safe UI display, cleanup on unload, no sync/network/scanner behavior, and report honesty.

## Allowed files

- apps/haze-obsidian-plugin/src/** only for small review fixes
- apps/haze-obsidian-plugin/docs/** only for small review fixes
- apps/haze-obsidian-plugin/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace apps/haze-obsidian-plugin/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
