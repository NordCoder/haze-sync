# W1-OBS-P6C-RERUN — Obsidian remote materialization clean-code review

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for OBS-P6, not OBS-P6C. Therefore OBS-P6C-RERUN is not already complete.

## Context

OBS-P6 implementation is complete. Component CI for the code-bearing commit is green.

- workflow: Component CI
- workflow_run_id: 29034944309
- run_number: 698
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review OBS-P6 remote changes pull and safe materialization. Focus on cursor/pull-state persistence, hash and revision verification before materialization, local dirty-file detection, Vault API write boundaries, safe cursor/base updates, tombstone-without-hard-delete behavior, conflict queueing, echo suppression, and non-goal preservation.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Forbidden changes

No provider calls, semantic merge, hard delete, server route changes, Worktree behavior, workflow changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to OBS-P6C-RERUN.
