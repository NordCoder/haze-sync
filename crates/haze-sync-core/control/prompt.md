# W1-CORE-P8C — Core compatibility-fixture clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

CORE-P8 implementation and artifact-based CI correction are complete. Final fixture/test/docs CI is green.

- code_bearing_sha: c2229f75f4fed31215f3a6f4b7f21ac41155d8e8
- workflow: Component CI
- workflow_run_id: 29120367603
- run_number: 1529
- conclusion: success

## Read

Read the project process files, current Core control files, CORE-P8 fixture catalog, integration tests, compatibility documentation, fixer correction, accepted API fixture guidance, and PR diff. Do not read diagnostics artifacts unless a later fixer prompt explicitly requires them.

## Task

Review CORE-P8 compatibility fixtures and integration-contract examples.

Focus on:

- language-neutral deterministic fixture shape;
- strict canonical serialization and public-type roundtrips;
- semantic recomputation through public Core APIs;
- accepted-write, same-content, conflict-saved, hash mismatch, tombstone, delete guard, idempotency, cursor, and doctor examples;
- safe synthetic paths, IDs, headers, and absence of raw bytes or secrets;
- conflict-saved serialized-tag versus public-status documentation;
- stable semantic contracts versus internal details;
- downstream usability without API DTO duplication or private-Core coupling.

## Allowed files

- crates/haze-sync-core/tests/**
- crates/haze-sync-core/fixtures/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/src/** only if directly required to correct a fixture helper or public re-export
- crates/haze-sync-core/control/report.md

## Boundaries

No API DTO duplication, TypeScript generation, Server/Storage/adapter runtime integration, persistence/provider behavior, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

Source/test/fixture/docs review commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-core/control/report.md using REPORT_TYPE CLEAN_CODE_REVIEW and phase_id CORE-P8C.
