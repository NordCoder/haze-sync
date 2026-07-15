# W1-GDA-GDA-P1-CONFIG-MODE-NORMALIZATION

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 GDA-GDA-P1 Config Mode Normalization`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: implementation-worker
Phase: `GDA-GDA-P1-CONFIG-MODE-NORMALIZATION`

This is the only active GDrive phase. Do not start OAuth, HTTP transport, durable-state client, scheduling, status, Deployment or E2E work in parallel inside this component.

## Accepted baseline

- accepted component-local product SHA: `06a7051a7e14c1da45de8cf96a78658b59cb823e`;
- synchronized fan-in baseline: `f9a2da6eb9ac6f59b1ec18ae4d85eb51964f3cbe`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
- PR #50 remains open, draft and unmerged.

Fetch the actual branch head before editing because Orchestrator control-only commits follow the baseline.

## Fixed architecture

Implement; do not redesign:

1. `HAZE_GDRIVE_MODE` is the single authoritative V1 mode.
2. Allowed values are `disabled`, `dry_run`, `read_only`, `import_only`, `export_only`, `bidirectional`.
3. Existing `HAZE_GDRIVE_DRY_RUN` is compatibility debt only.
4. If the compatibility boolean is supplied, it may only represent `mode=dry_run`; every contradictory combination fails startup closed.
5. `read_only` is globally non-mutating observation, not an alias for `export_only`.
6. `disabled` performs no provider/Core reads or durable state mutation; safe local status only.
7. `dry_run` permits planning reads only and performs no Core/provider/durable-state mutation.
8. Rollout order remains disabled -> dry_run -> one direction -> verified opposite direction -> bidirectional after explicit operator approval.
9. Config, Debug, Display and errors must not reveal tokens, raw provider identifiers, raw cursor values or absolute secret paths.

## Task

Normalize the existing GDrive configuration and mode enforcement surface so later OAuth, HTTP and runtime phases consume one unambiguous typed contract.

Required deliverables:

- one typed authoritative mode model and parser;
- explicit capability/permission helpers for provider reads, Core reads, Core writes, provider writes/trash and durable-state mutation;
- fail-closed validation for contradictory legacy `HAZE_GDRIVE_DRY_RUN` combinations;
- documented deprecation behavior for the legacy boolean without silently preserving two authorities;
- validation of mode-dependent required config only where the existing component contract already defines it;
- safe startup/status summary that reports mode/category only;
- focused tests for every mode, contradiction, missing/invalid values and redaction;
- minimal docs/decision/log alignment.

Prefer removal of internal duplicated boolean state. Retain input compatibility only at the configuration boundary if required by accepted deployment placeholders.

## Allowed scope

- `crates/haze-gdrive-adapter/src/config.rs` or a focused config submodule;
- mode-related code in `src/main.rs`, `src/runtime.rs`, `src/lib.rs` only as necessary to consume the normalized contract;
- focused GDrive tests;
- `crates/haze-gdrive-adapter/docs/**`;
- GDrive control report.

## Forbidden

- Google SDK/client or OAuth refresh implementation;
- provider calls;
- Server/API HTTP client;
- Storage/direct DB access;
- mapping/cursor persistence;
- import/export execution changes beyond enforcing existing planner permissions;
- scheduler/long-running loop;
- status ingestion/API DTOs;
- Deployment/Compose files;
- sibling component changes;
- workflow changes;
- real secrets or credentials;
- merge, draft-state change, rebase, force-push or history rewrite.

If existing planner modules require a mechanical mode-consumption adjustment, keep it minimal and do not add new planner behavior.

## Validation

Create code/test/doc commits without CI skip. Control/report-only commits may use CI skip.

Required exact final code-bearing SHA evidence:

- cargo fmt/check/test/clippy through Component CI;
- all existing GDrive tests remain green;
- new exhaustive mode/legacy/redaction tests green;
- PR #50 remains open, draft and unmerged.

## Report

Write only `crates/haze-gdrive-adapter/control/report.md`.

Set:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: GDA-GDA-P1-CONFIG-MODE-NORMALIZATION`;
- `chat_name: gdrive-adapter — W1 GDA-GDA-P1 Config Mode Normalization`.

Allowed statuses:

- `SELF_ACCEPT`;
- `SELF_ACCEPT_PENDING_CI`;
- `SELF_NEEDS_FIX`;
- `BLOCKED_BY_CONTRACT`;
- `BLOCKED_BY_DEPENDENCY`;
- `BLOCKED_BY_TOOLING`.

Record exact changed paths, authoritative mode contract, legacy compatibility behavior, capability matrix, redaction evidence, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT or merge readiness.
