# W1-FIX-OBS-P9-NODE-CI — Obsidian Node validation correction

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

The authorized merge-conflict resolution is complete.

- merge_commit: 0cf1e56e759824761ce608a45b25317d963b2257
- PR #51 mergeable: true
- branch behind main: 0
- workflow: Component CI
- workflow_run_id: 29113089168
- run_number: 1439
- run_attempt: 1
- Rust job: success
- Obsidian Node validation job: failure
- artifact_id: 8235526638
- artifact_name: ci-diag__component-obsidian-plugin__wf-component-ci__run-29113089168__attempt-1
- artifact_expires_at: 2026-07-11T18:02:18Z

Use the diagnostics artifact as source of truth.

## Artifact-proven failure

`node-test`, `node-typecheck`, and `node-build` all fail from the same TypeScript error:

```text
apps/haze-obsidian-plugin/tests/api-client.test.ts:23
TS2322: Type 'Response | Promise<Response>' is not assignable to type 'Promise<Response>'.
```

The helper currently permits synchronous or asynchronous handlers, while `HttpTransport` requires a `Promise<Response>` return.

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, current control files, apps/haze-obsidian-plugin/tests/api-client.test.ts, the `HttpTransport` declaration, package scripts, merged Component CI workflow, PR diff, and every file in artifact 8235526638: summary.md, manifest.json, failure markers, and logs.

## Task

Apply only the minimum correction needed to make the test helper satisfy `HttpTransport` while preserving support for both synchronous and asynchronous test handlers. Prefer normalizing the handler result to `Promise<Response>` at the transport boundary rather than weakening the production transport type.

Then allow Component CI to run and verify:

- Rust workspace job remains green;
- npm ci succeeds;
- plugin tests succeed;
- plugin typecheck succeeds;
- plugin build succeeds;
- Node diagnostics finalization succeeds.

If a new check fails, do not broaden scope beyond artifact-proven evidence; report the exact new run and artifact for Orchestrator triage.

## Allowed files

- apps/haze-obsidian-plugin/tests/api-client.test.ts
- apps/haze-obsidian-plugin/control/report.md

Only if the artifact proves the declared production contract itself is wrong:

- apps/haze-obsidian-plugin/src/api-client/**

## Boundaries

No workflow changes, dependency upgrades, product behavior changes, API vocabulary changes, generated output, test deletion, assertion weakening, PR lifecycle action, merge, sibling changes, or secrets.

## CI trigger policy

The test/source correction commit must not skip CI. A final report-only commit may skip CI only after the code-bearing correction has observable workflow evidence.

## Report

Write only apps/haze-obsidian-plugin/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-OBS-P9-NODE-CI.
