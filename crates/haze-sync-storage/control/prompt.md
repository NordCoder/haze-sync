# W1-FIX-STOR-GDA-P1-RUST-WORKSPACE-CI

Before starting, name this worker chat exactly:

`storage — W1 FIX-STOR-GDA-P1 Rust Workspace CI`

Repository: `NordCoder/haze-sync`
Component: storage
Path: `crates/haze-sync-storage`
Branch/ref: `component/storage`
PR: #47
Role: fixer-worker
Phase: `FIX-STOR-GDA-P1-RUST-WORKSPACE-CI`

This is a focused fixer loop for the existing `STOR-GDA-P1-DURABLE-STATE` implementation. Do not start a new Storage product phase.

## Authoritative failing candidate

- code-bearing SHA: `4f539e32ae8768aeeb9cda11f388745c3771496c`;
- Component CI run: `29421594032`;
- run number: `1987`;
- run attempt: `1`;
- overall conclusion: failure;
- Storage PostgreSQL verification: success;
- Rust workspace job: failure;
- failed job id: `87373222668`;
- diagnostics artifact id: `8345471214`;
- diagnostics artifact name: `ci-diag__component-storage__wf-component-ci__run-29421594032__attempt-1`.

## Required reading

Read:

- Project Source `implementation-manifest.md`;
- Project Source `fixer-worker-prompt.md`;
- Project Source `report-template.md`;
- current Storage contract, implementation plan, implementation log and dependency map;
- this active prompt from `ref=component/storage`;
- archived implementation report pinned by blob `b3a0113b6db03e8ac410e9ca0708a38ae4f22d5c` if needed.

## Diagnostics protocol

Use the diagnostics artifact as the primary source.

1. Download artifact id `8345471214`.
2. Read:
   - `ci-diagnostics/summary.md`;
   - `ci-diagnostics/manifest.json`;
   - every log listed under `failed_checks`.
3. Do not guess the failure from the finalizer step or from partial metadata.
4. Raw GitHub job logs are allowed only if the artifact is missing, malformed, expired or does not contain the required failed-check log, and the report must state the fallback.
5. If the artifact cannot be read, report `FIX_BLOCKED_BY_LOGS`.

## Task

Fix only the proven cause of the Rust workspace failure.

Preserve the accepted implementation surface:

- migration `0011_gdrive_durable_state.sql`;
- caller-owned transaction repositories;
- state-version compare-and-commit;
- contiguous opaque cursor advancement;
- monotonic Core export checkpoint;
- atomic mapping/echo/delete-candidate/operation facts;
- deterministic operation replay/conflict behavior;
- adapter isolation;
- redaction and safe errors;
- green PostgreSQL verification and migration evidence.

Do not weaken tests, remove evidence checks, redesign the Storage contract, or broaden into API, Server, GDrive Adapter, Core, Deployment, CLI, Obsidian or workflow changes.

## Scope

Allowed changes are the minimum Storage-owned files needed to correct the artifact-proven failure, plus focused tests and Storage docs only if the fix changes an internal documented contract.

Forbidden:

- sibling component files;
- `.github/workflows/**`;
- new product behavior unrelated to the failure;
- accepted migration rewrites unless the artifact proves a concrete compile/test defect requiring a minimal correction and data safety remains preserved;
- deleting or weakening tests;
- secrets, provider calls, direct Adapter DB access;
- merge, draft-state change, rebase, force-push or history rewrite.

## Validation

Create code/test fixes without CI skip.

Obtain a new full Component CI run on the exact final code-bearing SHA. Required:

- Rust workspace green;
- Storage PostgreSQL verification green;
- migration and strict evidence checks remain green;
- PR #47 remains open, draft and unmerged.

Do not accept a rerun of the unchanged failing SHA as proof of a code fix unless the artifact proves a transient infrastructure-only failure. If transient, record exact evidence and obtain a green rerun.

## Report

Write only `crates/haze-sync-storage/control/report.md`.

Set:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-STOR-GDA-P1-RUST-WORKSPACE-CI`;
- `chat_name: storage — W1 FIX-STOR-GDA-P1 Rust Workspace CI`.

Allowed statuses:

- `FIX_COMPLETE`;
- `FIX_NEEDS_MORE`;
- `FIX_BLOCKED_BY_LOGS`;
- `FIX_BLOCKED_BY_CONTRACT`;
- `FIX_BLOCKED_BY_TOOLING`.

Record artifact metadata and files read, exact failure cause, minimal changed paths, checks, final code-bearing SHA and exact full CI evidence. Do not claim CLEAN_ACCEPT or merge readiness. A Storage clean/DB review follows only after green CI.
