# W1-FIX-API-GDA-P1-CI

Before starting, name this worker chat exactly:

`api — W1 FIX-API-GDA-P1 CI`

Repository: `NordCoder/haze-sync`
Component: api
Path: `crates/haze-sync-api`
Branch/ref: `component/api`
PR: #44
Role: fixer-worker
Phase: `FIX-API-GDA-P1-CI`

This is the fixer loop for the existing `API-GDA-P1-CONTRACTS` phase. Do not begin another API product phase.

## Failing candidate

- exact code-bearing SHA: `8354b7d0b9bb9152e5609d36814222741b70d14e`;
- Component CI run: `29437624209`, run number `2009`, attempt `2`;
- visible cargo fmt/check/test/clippy steps: success;
- diagnostics finalizer: failure;
- artifact id: `8352138123`;
- artifact name: `ci-diag__component-api__wf-component-ci__run-29437624209__attempt-2`;
- implementation report blob: `490921fb423bb0c6119bb966cab49f72d6f0e619`.

## Required diagnostics protocol

Download artifact `8352138123` and read:

- `summary.md`;
- `manifest.json`;
- every failure marker and log named by `failed_checks`.

Use raw job logs only if the artifact is missing, expired, malformed or incomplete. Do not infer the cause from the finalizer step alone. If evidence cannot be read, report `FIX_BLOCKED_BY_LOGS`.

## Task

Fix only the artifact-proven cause. Preserve the implemented passive API surface:

- bounded private GDrive state snapshot DTOs;
- sanitized admin summary;
- strict compare-and-commit request and outcomes;
- matching-adapter authorization metadata and admin read-only behavior;
- mandatory redacted idempotency metadata;
- state/cursor/mapping/idempotency error vocabulary;
- private raw-cursor boundary and Debug/Display/error redaction;
- deterministic compatibility fixture;
- no Axum registration, Server execution, Storage calls, Core policy, provider/OAuth or scheduler behavior.

Do not weaken tests, alter workflows, edit sibling components, redesign accepted Storage semantics, add status/operator contracts, merge, rebase, force-push or change PR draft state.

## Validation

Create any code/test fix without CI skip. Obtain a new full exact-SHA Component CI run with fmt, check, test, clippy and diagnostics finalization green. PR #44 remains open, draft and unmerged.

## Report

Write only `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-API-GDA-P1-CI`;
- `chat_name: api — W1 FIX-API-GDA-P1 CI`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record artifact files read, exact cause, minimum changed paths, final code-bearing SHA and full exact-SHA CI evidence. Do not claim CLEAN_ACCEPT or merge readiness.
