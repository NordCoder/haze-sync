# W1-API-P5C — API conflict/delete clean-code review

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P5 implementation and CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: 8a80d45685d29a681d37e0861ec8ea1ba2e734c7
- workflow: Component CI
- workflow_run_id: 29079842861
- run_number: 892
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review API-P5 conflict and delete route-helper contracts plus the formatting fixer.

Focus areas:

- bounded conflict-list status parsing and safe rejection;
- complete supported conflict action vocabulary and payload validation;
- conflict list/detail/resolve DTO roundtrips and safe metadata;
- delete metadata parsing for known/null base revision and idempotency data;
- verified-principal requirement and safe missing-principal mapping;
- stable tombstoned/not_found/rejected/guard-blocked response vocabulary;
- source compatibility of passive route-helper request parts;
- preservation of API passivity.

## Allowed files

- crates/haze-sync-api/src/routes/conflicts/**
- crates/haze-sync-api/src/routes/delete/**
- crates/haze-sync-api/src/dto/conflicts/**
- crates/haze-sync-api/src/dto/files/** when directly relevant
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Forbidden changes

No route runtime implementation, persistence/database/object-store calls, conflict resolution execution, tombstone creation, provider/worktree deletion behavior, mass-delete execution, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to API-P5C.
