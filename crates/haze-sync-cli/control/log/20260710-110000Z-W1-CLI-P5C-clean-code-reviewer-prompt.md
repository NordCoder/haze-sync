# Archived active prompt

component: cli
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: CLI-P5C
agent_role: clean-code-reviewer
source_path: crates/haze-sync-cli/control/prompt.md
source_sha: cdda7c331c2c3ff56b62a286b8d1e9fef670b3e9

# W1-CLI-P5C — CLI live-doctor clean-code review

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P5 implementation and CI fixer are complete. Final code-bearing Component CI is green.

- code_bearing_sha: 97ef1ae1c6995aff71d364d303d0cdbea040de49
- workflow_run_id: 29081982496
- run_number: 966
- conclusion: success

## Read

Read process sources, component docs/control files, current source/tests, accepted Server/API/Core diagnostic contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Review CLI-P5 live doctor integration plus the CI correction.

Focus on offline-by-default behavior, explicit live mode, accepted public diagnostic surfaces, skipped/not-run honesty, safe partial failures, output redaction, predictable exit codes, client abstraction boundaries, and non-goal preservation.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Boundaries

No repair behavior, direct DB/object-store/provider access, new Server/API surfaces, token lifecycle work, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-cli/control/report.md. Use REPORT_TYPE CLEAN_CODE_REVIEW and phase_id CLI-P5C.
