# W1-GDA-P7C — GDrive export planner clean-code review

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P7 implementation and artifact-based CI correction are complete. Final source CI is green.

- code_bearing_sha: 7ce5dd9493d2325c5995ee0a5908fa7365574a4e
- workflow: Component CI
- workflow_run_id: 29113718421
- run_number: 1455
- conclusion: success

STOR-P8 persistence repositories are now accepted, but concrete cross-component wiring remains outside this component-local review.

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current GDA-P7 export source/tests, fixer changes, accepted Core/API change and tombstone contracts, accepted Storage mapping/state boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review GDA-P7 Core-to-Drive export planning and fake-first apply runner.

Focus on:

- source path, revision, declared size, and SHA-256 verification before provider mutation;
- create/update/trash selection and safe missing-mapping tombstone behavior;
- export_only, bidirectional, dry_run, and non-exporting modes;
- stable operation IDs, replay accounting, and redacted Debug/public output;
- provider preconditions and conflict/rate-limit/transient/auth classifications;
- mapping, echo, and cursor persistence ordering under partial failure and retry;
- injected Core/provider/state boundaries and readiness for later Storage/Server fan-in;
- module/public API shape, focused tests, and non-goal preservation.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No Core policy changes, Drive hard delete, live Google credentials/provider wiring, direct Storage/DB ownership, concrete Server/API transport, background scheduler, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id GDA-P7C.
