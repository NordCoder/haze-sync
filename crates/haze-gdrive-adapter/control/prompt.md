# W1-FIX-GDA-GDA-P1-REVIEW

Before starting, name this worker chat exactly:

`gdrive-adapter — W1 FIX-GDA-GDA-P1 Review`

Repository: `NordCoder/haze-sync`
Component: gdrive-adapter
Path: `crates/haze-gdrive-adapter`
Branch/ref: `component/gdrive-adapter`
PR: #50
Role: fixer-worker
Phase: `FIX-GDA-GDA-P1-REVIEW`

This is a focused fixer for the existing `GDA-GDA-P1-CONFIG-MODE-NORMALIZATION` phase. Do not start the next GDrive product phase.

## Review target and finding

- reviewed code-bearing SHA: `9bbbe6a3b6d3ea9935cb2b64390af042b4837c05`;
- clean-review report blob: `59d01050a4b2a59ce47c392b6fb3711a873a1574`;
- authoritative green CI before review finding: run `29435042810`, number `2001`.

The review found one high-severity invariant defect:

- `AdapterConfig` stores public `mode` and independent public `dry_run` values;
- `StartupStatus` copies/displays `dry_run` independently;
- direct construction can create `ImportOnly + dry_run=true`;
- an existing runtime test currently blesses that contradiction.

## Required fix

Eliminate the second mutable mode authority.

Accepted outcome:

1. `AdapterMode` remains the only stored authority.
2. Legacy `HAZE_GDRIVE_DRY_RUN` remains only a boundary-validation compatibility input.
3. Any dry-run view is derived from `mode.is_dry_run_mode()` or equivalent and cannot be assigned independently.
4. `StartupStatus` must not store a contradictory independent boolean; derive any compatibility display/accessor solely from its mode.
5. Tests must reject or make impossible contradictory direct construction and must no longer expect `ImportOnly` to report dry-run behavior.
6. Preserve all six modes, capability matrix, strict aliases, fail-closed environment validation and redaction.

## Scope

Allowed only where necessary:

- `crates/haze-gdrive-adapter/src/config.rs`;
- `crates/haze-gdrive-adapter/src/runtime.rs`;
- focused existing/new GDrive tests;
- minimal docs only if the public internal contract wording changes;
- control report.

Forbidden:

- OAuth/provider client, HTTP, Storage/persistence, scheduler expansion, public status API, Deployment or sibling work;
- workflow changes;
- weakening tests;
- unrelated refactors;
- merge, rebase, force-push or PR draft-state change.

## Validation

Create product/test changes without CI skip. Obtain a new full exact-SHA Component CI run with cargo fmt/check/test/clippy and diagnostics finalization green. PR #50 remains open, draft and unmerged.

## Report

Write only `crates/haze-gdrive-adapter/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-GDA-GDA-P1-REVIEW`;
- `chat_name: gdrive-adapter — W1 FIX-GDA-GDA-P1 Review`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record exact changed paths, how duplicate authority was eliminated, updated invariant tests, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT; a repeat clean review follows.
