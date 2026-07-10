# W1-OBS-P9 — Compatibility, packaging, and E2E readiness

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P8 implementation and clean-code review are accepted. API-P7 compatibility fixtures and API-P7C clean-code review are now accepted with green CI, so the previous OBS-P9 dependency gate is satisfied.

- obsidian_code_bearing_sha: f54a79b4a35c38d7b818cc323af0e166aa47b2c2
- obsidian_ci_run_id: 29084771114
- api_fixture_code_bearing_sha: 3109c0fd9b456ca5fd8db099cd83843dae44cef9
- api_fixture_ci_run_id: 29093081652
- api_fixture_ci_conclusion: success

The next implementation phase is OBS-P9 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, the accepted API compatibility fixture and guidance, current TypeScript DTO mirrors/client/tests, packaging configuration, plugin manifest, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement OBS-P9: Compatibility, packaging, and E2E readiness.

- consume the accepted API-P7 language-neutral fixture in TypeScript compatibility tests;
- align plugin DTO mirrors with exact canonical API fields and closed vocabularies;
- remove permissive fallback types where fixture-backed closed unions exist;
- add mocked API client tests for representative scan, push, pull, conflict, delete, status, and doctor flows where current abstractions support them;
- verify or document typecheck/build expectations through available CI/tooling;
- document local installation and test-vault/test-server procedures;
- make generated bundle tracking policy explicit without committing generated artifacts unless accepted;
- prepare deterministic local E2E scenarios using synthetic data only.

## Allowed files

- apps/haze-obsidian-plugin/**

## Non-goals

No production marketplace release, real user vault data, secrets in fixtures, generated artifacts committed without explicit policy, API fixture ownership changes, Server/Core runtime changes, provider behavior, workflow changes, or sibling component changes.

## CI trigger policy

Source/test/docs/config commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only apps/haze-obsidian-plugin/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id OBS-P9.
