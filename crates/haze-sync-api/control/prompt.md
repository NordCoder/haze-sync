# W1-FIX-API-P8-CI — Diagnose and fix Component CI finalizer failure

Before starting, name this worker chat exactly:

`api — W1 API-P8 CI Diagnostics Fix`

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: fixer-worker
Phase: FIX-API-P8-CI

Do not merge, change draft state, rewrite history, modify sibling branches, or perform unrelated cleanup.

## Candidate

- API-P8 final code-bearing SHA: `8eb6e0ce44612e1e2f111415026297df8fb1d82b`;
- implementation report commit: `8f430d8c05b5151b91039c280ea20cfe2b382f3e`;
- implementation report blob: `390fa0b676b57a1796b840eb0d63f9ebae2599b4`;
- Component CI run: `29313793376`, run number `1922`;
- attempts: 1 and 2;
- exact head SHA: `8eb6e0ce44612e1e2f111415026297df8fb1d82b`;
- cargo fmt/check/test/clippy: success on both attempts;
- sole failed step: `Finalize CI diagnostics`;
- diagnostics upload: success.

## Authoritative diagnostics artifact

- artifact id: `8303189576`;
- artifact name: `ci-diag__component-api__wf-component-ci__run-29313793376__attempt-2`;
- digest: `sha256:03e85bcdf1b7d796eadb6415767bab60cf1399f4565440f397f7f585c03a00b6`;
- expires: `2026-07-15T07:16:40Z`;
- currently unexpired.

Download and inspect this artifact first. Read:

1. `summary.md`;
2. `manifest.json`;
3. every log named by `failed_checks` in the manifest/summary.

Treat those files as authoritative. Do not guess from the workflow UI or from the implementation report.

## Task

Find and apply the smallest evidence-backed correction required for the finalizer failure.

Rules:

1. If diagnostics identify only workflow/script tooling, preserve API-P8 product files exactly.
2. If diagnostics identify an API-P8-generated input that violates the finalizer contract, change only that exact API/control input and explain why all Rust checks still passed.
3. Do not alter public API-P8 vocabulary, DTO semantics, fixtures, auth rules or passive boundaries unless diagnostics prove a product defect.
4. Do not suppress, skip, ignore or force-success the failing finalizer.
5. Do not weaken diagnostics, remove a check, or convert a failure into a warning.
6. Do not use raw job logs unless the artifact is missing, corrupt, incomplete, or internally inconsistent. If artifact evidence is insufficient, report `FIX_BLOCKED_BY_LOGS` without guessing.
7. After the correction, create a real code/tooling-bearing commit without CI skip and obtain authoritative Component CI success on its exact SHA.
8. If the correction is repository-wide workflow tooling, limit changes to the exact script/workflow path named by diagnostics and note downstream branch implications; do not fan it out to sibling branches yourself.

## Allowed scope

Only when directly supported by artifact evidence:

- `.github/scripts/ci-finalize.sh`;
- `.github/scripts/ci-run.sh`;
- `.github/workflows/component-ci.yml`;
- exact API control/fixture/test file identified by diagnostics;
- `crates/haze-sync-api/control/report.md`.

API-P8 product files are otherwise protected:

- `crates/haze-sync-api/src/dto/worktree.rs`;
- `crates/haze-sync-api/src/routes/worktree.rs`;
- Worktree fixture/docs/tests and module exports.

Forbidden:

- Server, Worktree, Storage, Core, CLI or Deployment product changes;
- migrations, provider/runtime/database/filesystem work;
- route registration or background behavior;
- broad workflow refactors;
- formatting-only product edits;
- unrelated cleanup.

## Completion

Write `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: FIX`;
- `phase_id: FIX-API-P8-CI`;
- `chat_name: api — W1 API-P8 CI Diagnostics Fix`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE_WORK`, `FIX_BLOCKED_BY_LOGS`, `FIX_BLOCKED_BY_SCOPE`, or `FIX_BLOCKED_BY_TOOLING`.

Record:

- artifact files read;
- exact failed check and root cause quoted/paraphrased from diagnostics;
- changed paths and why each was necessary;
- whether API-P8 product blobs remained unchanged;
- final code/tooling SHA;
- authoritative exact-SHA CI run, number, attempt and conclusions for every step.

Do not claim CLEAN_ACCEPT or begin functional review/Server fan-in/CLI-P6A. A focused API-P8 review follows after green CI.
